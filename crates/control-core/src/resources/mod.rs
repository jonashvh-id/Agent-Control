//! Resource registry, encrypted metadata and current per-agent authority.
use crate::{
    error::{Error, Result},
    models::Policy,
    persistence::{id, now},
    security::keys::Keys,
};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use sqlx::{Row, SqliteConnection, SqlitePool};
use std::sync::Arc;
pub mod policy;
#[derive(Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Metadata {
    pub provider_id: String,
    pub name: String,
    pub kind: String,
}
#[derive(Clone)]
pub struct Store {
    pub pool: SqlitePool,
    pub keys: Arc<Keys>,
}
impl Store {
    pub async fn list(&self, connector: &str) -> Result<Vec<Value>> {
        let rows=sqlx::query("SELECT id,enabled,metadata_ciphertext FROM resources WHERE connector=? ORDER BY created_at").bind(connector).fetch_all(&self.pool).await?;
        rows.into_iter()
            .map(|r| {
                let id: String = r.get("id");
                let mut v = self
                    .keys
                    .open_private(&format!("resource:{id}"), r.get("metadata_ciphertext"))?;
                v["id"] = json!(id);
                v["enabled"] = json!(r.get::<i64, _>("enabled") == 1);
                v["connector"] = json!(connector);
                Ok(v)
            })
            .collect()
    }
    pub async fn add(&self, connector: &str, human: &str, metadata: Metadata) -> Result<String> {
        if metadata.provider_id.len() > 256
            || metadata.name.chars().count() > 1024
            || metadata.kind.len() > 40
        {
            return Err(Error::new("invalid_resource_metadata", 422));
        }
        let mut db = self.pool.begin_with("BEGIN IMMEDIATE").await?;
        let authorized: Option<i64> =
            sqlx::query_scalar("SELECT 1 FROM humans WHERE id=? AND enabled=1 AND role='admin'")
                .bind(human)
                .fetch_optional(&mut *db)
                .await?;
        if authorized.is_none() {
            return Err(Error::new("admin_required", 403));
        }
        let total: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM resources WHERE connector=?")
            .bind(connector)
            .fetch_one(&mut *db)
            .await?;
        if total >= 100 {
            return Err(Error::new("resource_limit_reached", 409));
        }
        let count: i64 =
            sqlx::query_scalar("SELECT COUNT(*) FROM resources WHERE connector=? AND enabled=1")
                .bind(connector)
                .fetch_one(&mut *db)
                .await?;
        if count >= 20 {
            return Err(Error::new("resource_limit_reached", 409));
        }
        let id = id();
        let ciphertext = self.keys.seal_private(
            &format!("resource:{id}"),
            &serde_json::to_value(&metadata).unwrap(),
        )?;
        sqlx::query("INSERT INTO resources VALUES(?,?,?,?,1,?,?,?)")
            .bind(&id)
            .bind(connector)
            .bind(
                self.keys
                    .hash(&format!("resource:{connector}:{}", metadata.provider_id)),
            )
            .bind(ciphertext)
            .bind(human)
            .bind(now())
            .bind(now())
            .execute(&mut *db)
            .await
            .map_err(|_| Error::new("resource_already_registered", 409))?;
        sqlx::query("UPDATE connection_secrets SET verification='{}',resource_revision=resource_revision+1 WHERE connector=?").bind(connector).execute(&mut *db).await?;
        crate::audit::record(
            &mut db,
            crate::audit::Event {
                actor_kind: "human",
                actor_id: human,
                connector,
                resource_ref: &id,
                action: "resource.registered",
                decision: "allow",
                ..Default::default()
            },
        )
        .await?;
        db.commit().await?;
        Ok(id)
    }
    pub async fn get(&self, connector: &str, id: &str) -> Result<Metadata> {
        let ciphertext: String = sqlx::query_scalar(
            "SELECT metadata_ciphertext FROM resources WHERE id=? AND connector=? AND enabled=1",
        )
        .bind(id)
        .bind(connector)
        .fetch_optional(&self.pool)
        .await?
        .ok_or(Error::new("resource_permission_denied", 403))?;
        serde_json::from_value(
            self.keys
                .open_private(&format!("resource:{id}"), &ciphertext)?,
        )
        .map_err(|_| Error::new("private_store_unavailable", 503))
    }
}
pub async fn available(
    db: &mut SqliteConnection,
    agent: &str,
    connector: &str,
) -> Result<Vec<Value>> {
    Ok(sqlx::query("SELECT r.id,p.mode FROM resources r JOIN resource_policies p ON p.resource_id=r.id WHERE p.agent_id=? AND r.connector=? AND r.enabled=1 AND p.mode!='deny'").bind(agent).bind(connector).fetch_all(db).await?.into_iter().map(|r|json!({"id":r.get::<String,_>("id"),"mode":r.get::<String,_>("mode")})).collect())
}
pub async fn authorize(
    db: &mut SqliteConnection,
    agent: &str,
    connector: &str,
    parameters: &Value,
    base: Policy,
) -> Result<Policy> {
    let root = parameters["resource_id"]
        .as_str()
        .ok_or(Error::new("resource_permission_denied", 403))?;
    let mode:Option<String>=sqlx::query_scalar("SELECT p.mode FROM resources r JOIN resource_policies p ON p.resource_id=r.id WHERE p.agent_id=? AND r.id=? AND r.connector=? AND r.enabled=1").bind(agent).bind(root).bind(connector).fetch_optional(db).await?;
    let mode = match mode.as_deref() {
        Some("allow") => Policy::Allow,
        Some("ask") => Policy::Ask,
        _ => return Err(Error::new("resource_permission_denied", 403)),
    };
    Ok(policy::restrict(base, mode))
}
pub async fn labels(db: &mut SqliteConnection, keys: &Keys, resources: &mut [Value]) -> Result<()> {
    for resource in resources {
        let id = resource["id"]
            .as_str()
            .ok_or(Error::new("resource_not_found", 404))?;
        let cipher: String =
            sqlx::query_scalar("SELECT metadata_ciphertext FROM resources WHERE id=?")
                .bind(id)
                .fetch_one(&mut *db)
                .await?;
        let metadata = keys.open_private(&format!("resource:{id}"), &cipher)?;
        resource["name"] = metadata["name"].clone();
    }
    Ok(())
}
pub async fn observe(
    db: &mut SqliteConnection,
    keys: &Keys,
    resource: &str,
    objects: Vec<Metadata>,
) -> Result<Vec<String>> {
    if objects.len() > 20 {
        return Err(Error::new("invalid_connector_result", 502));
    }
    let mut refs = vec![];
    for object in objects {
        if object.provider_id.len() > 256
            || object.name.chars().count() > 1024
            || object.kind.len() > 40
        {
            return Err(Error::new("invalid_connector_result", 502));
        }
        let reference = keys.hash(&format!("object:{resource}:{}", object.provider_id));
        let ciphertext = keys.seal_private(
            &format!("object:{reference}"),
            &serde_json::to_value(&object).unwrap(),
        )?;
        sqlx::query("INSERT INTO resource_objects VALUES(?,?,?) ON CONFLICT(ref) DO UPDATE SET metadata_ciphertext=excluded.metadata_ciphertext").bind(&reference).bind(resource).bind(ciphertext).execute(&mut *db).await?;
        refs.push(reference);
    }
    Ok(refs)
}
