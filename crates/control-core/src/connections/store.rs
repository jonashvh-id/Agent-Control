//! Encrypted provider state; no transport or provider-specific token handling.
use crate::{
    error::{Error, Result},
    persistence::now,
    security::keys::Keys,
};
use serde_json::{Value, json};
use sqlx::{Row, SqlitePool};
use std::sync::Arc;
#[derive(Clone)]
pub struct Store {
    pub pool: SqlitePool,
    pub keys: Arc<Keys>,
}
impl Store {
    pub async fn read(&self, connector: &str) -> Result<(String, Option<Value>, i64, Value)> {
        let row = sqlx::query("SELECT * FROM connection_secrets WHERE connector=?")
            .bind(connector)
            .fetch_optional(&self.pool)
            .await?;
        match row {
            None => Ok(("disconnected".into(), None, 0, json!({}))),
            Some(row) => {
                let ciphertext: String = row.get("ciphertext");
                Ok((
                    row.get("state"),
                    if ciphertext.is_empty() {
                        None
                    } else {
                        Some(
                            self.keys
                                .open_private(&format!("connection:{connector}"), &ciphertext)?,
                        )
                    },
                    row.get("generation"),
                    serde_json::from_str(row.get("verification"))
                        .map_err(|_| Error::new("private_store_unavailable", 503))?,
                ))
            }
        }
    }
    pub async fn replace(
        &self,
        connector: &str,
        value: &Value,
        human: &str,
        session_hash: &str,
    ) -> Result<()> {
        let ciphertext = self
            .keys
            .seal_private(&format!("connection:{connector}"), value)?;
        let mut db = self.pool.begin_with("BEGIN IMMEDIATE").await?;
        let authorized:Option<i64>=sqlx::query_scalar("SELECT 1 FROM sessions s JOIN humans h ON h.id=s.human_id WHERE s.token_hash=? AND h.id=? AND s.expires_at>? AND h.enabled=1 AND h.role='admin'").bind(session_hash).bind(human).bind(crate::persistence::epoch()).fetch_optional(&mut *db).await?;
        if authorized.is_none() {
            return Err(Error::new("human_authentication_required", 401));
        }
        sqlx::query("INSERT INTO connection_secrets(connector,state,ciphertext,generation,updated_at) VALUES(?,'connected',?,1,?) ON CONFLICT(connector) DO UPDATE SET state='connected',ciphertext=excluded.ciphertext,generation=generation+1,verification='{}',updated_at=excluded.updated_at")
   .bind(connector).bind(ciphertext).bind(now()).execute(&mut *db).await?;
        crate::audit::record(
            &mut db,
            crate::audit::Event {
                actor_kind: "human",
                actor_id: human,
                connector,
                action: "connection.oauth_connected",
                decision: "allow",
                ..Default::default()
            },
        )
        .await?;
        db.commit().await?;
        Ok(())
    }
    pub async fn refresh(&self, connector: &str, generation: i64, value: &Value) -> Result<()> {
        let ciphertext = self
            .keys
            .seal_private(&format!("connection:{connector}"), value)?;
        let mut db = self.pool.begin_with("BEGIN IMMEDIATE").await?;
        if sqlx::query("UPDATE connection_secrets SET ciphertext=?,updated_at=? WHERE connector=? AND state='connected' AND generation=?")
   .bind(ciphertext).bind(now()).bind(connector).bind(generation).execute(&mut *db).await?.rows_affected()!=1 {return Err(Error::new("connection_changed",409));}
        crate::audit::record(
            &mut db,
            crate::audit::Event {
                actor_kind: "system",
                connector,
                action: "connection.token_renewed",
                decision: "allow",
                ..Default::default()
            },
        )
        .await?;
        db.commit().await?;
        Ok(())
    }
    pub async fn clear(&self, connector: &str, state: &str) -> Result<()> {
        if !matches!(state, "disconnected" | "reauth_required") {
            return Err(Error::new("invalid_connection_state", 400));
        }
        let mut db = self.pool.begin_with("BEGIN IMMEDIATE").await?;
        sqlx::query("UPDATE connection_secrets SET ciphertext='',state=?,generation=generation+1,verification='{}',updated_at=? WHERE connector=?").bind(state).bind(now()).bind(connector).execute(&mut *db).await?;
        sqlx::query("DELETE FROM oauth_attempts WHERE connector=?")
            .bind(connector)
            .execute(&mut *db)
            .await?;
        sqlx::query("UPDATE connector_state SET enabled=0 WHERE id=?")
            .bind(connector)
            .execute(&mut *db)
            .await?;
        crate::audit::record(
            &mut db,
            crate::audit::Event {
                actor_kind: "system",
                connector,
                action: if state == "reauth_required" {
                    "connection.reauthentication_required"
                } else {
                    "connection.local_access_removed"
                },
                decision: state,
                ..Default::default()
            },
        )
        .await?;
        db.commit().await?;
        Ok(())
    }
    pub async fn verification(&self, connector: &str, value: &Value) -> Result<()> {
        sqlx::query("UPDATE connection_secrets SET verification=?,updated_at=? WHERE connector=?")
            .bind(value.to_string())
            .bind(now())
            .bind(connector)
            .execute(&self.pool)
            .await?;
        Ok(())
    }
    pub async fn revision(&self, connector: &str) -> Result<(i64, i64)> {
        sqlx::query_as("SELECT generation,resource_revision FROM connection_secrets WHERE connector=? AND state='connected'").bind(connector).fetch_optional(&self.pool).await?.ok_or(Error::new("google_connection_required",403))
    }
    pub async fn verification_checked(
        &self,
        connector: &str,
        value: &Value,
        generation: i64,
        revision: i64,
    ) -> Result<()> {
        let changed=sqlx::query("UPDATE connection_secrets SET verification=? WHERE connector=? AND state='connected' AND generation=? AND resource_revision=?").bind(value.to_string()).bind(connector).bind(generation).bind(revision).execute(&self.pool).await?.rows_affected();
        if changed != 1 {
            return Err(Error::new("connection_changed", 409));
        }
        Ok(())
    }
    pub async fn pending(&self, connector: &str, state: &str, cookie: &str) -> Result<()> {
        let present:Option<i64>=sqlx::query_scalar("SELECT 1 FROM oauth_attempts WHERE state_hash=? AND connector=? AND cookie_hash=? AND consumed=0 AND expires_at>?").bind(self.keys.hash(state)).bind(connector).bind(cookie).bind(crate::persistence::epoch()).fetch_optional(&self.pool).await?;
        if present.is_none() {
            return Err(Error::new("oauth_state_rejected", 403));
        }
        Ok(())
    }
    pub async fn consume(&self, connector: &str, state: &str, cookie_hash: &str) -> Result<Value> {
        let mut db = self.pool.begin_with("BEGIN IMMEDIATE").await?;
        let hash = self.keys.hash(state);
        let row=sqlx::query("SELECT o.* FROM oauth_attempts o JOIN sessions s ON s.token_hash=o.session_hash JOIN humans h ON h.id=o.human_id AND h.id=s.human_id WHERE o.state_hash=? AND o.connector=? AND o.cookie_hash=? AND o.consumed=0 AND o.expires_at>? AND s.expires_at>? AND h.enabled=1 AND h.role='admin'")
   .bind(&hash).bind(connector).bind(cookie_hash).bind(crate::persistence::epoch()).bind(crate::persistence::epoch()).fetch_optional(&mut *db).await?.ok_or(Error::new("oauth_state_rejected",403))?;
        let mut data = self.keys.open_private(
            &format!("oauth-attempt:{hash}"),
            row.get("verifier_ciphertext"),
        )?;
        data["human_id"] = json!(row.get::<String, _>("human_id"));
        data["session_hash"] = json!(row.get::<String, _>("session_hash"));
        sqlx::query("UPDATE oauth_attempts SET consumed=1 WHERE state_hash=? AND consumed=0")
            .bind(&hash)
            .execute(&mut *db)
            .await?;
        db.commit().await?;
        Ok(data)
    }
}
