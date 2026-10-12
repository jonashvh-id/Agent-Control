use crate::{
    App,
    audit::{self, Event},
    error::{Error, Result},
    models::{Human, Policy},
    persistence::now,
};
pub fn restrict(tool: Policy, resource: Policy) -> Policy {
    if tool == Policy::Deny || resource == Policy::Deny {
        Policy::Deny
    } else if tool == Policy::Ask || resource == Policy::Ask {
        Policy::Ask
    } else {
        Policy::Allow
    }
}
pub async fn set(
    app: &App,
    actor: &Human,
    agent: &str,
    resource: &str,
    mode: Policy,
) -> Result<()> {
    let mut db = app.db.write().await?;
    crate::agents::get(&mut db, agent).await?;
    let connector: String = sqlx::query_scalar(
        "SELECT connector FROM resources WHERE id=? AND (enabled=1 OR ?='deny')",
    )
    .bind(resource)
    .bind(mode.as_str())
    .fetch_optional(&mut *db)
    .await?
    .ok_or(Error::new("resource_not_found", 404))?;
    sqlx::query("INSERT INTO resource_policies VALUES(?,?,?,?,?) ON CONFLICT(agent_id,resource_id) DO UPDATE SET mode=excluded.mode,updated_by=excluded.updated_by,updated_at=excluded.updated_at").bind(agent).bind(resource).bind(mode.as_str()).bind(&actor.id).bind(now()).execute(&mut *db).await?;
    audit::record(
        &mut db,
        Event {
            actor_kind: "human",
            actor_id: &actor.id,
            agent_id: agent,
            connector: &connector,
            resource_ref: resource,
            action: "resource.policy",
            decision: mode.as_str(),
            ..Default::default()
        },
    )
    .await?;
    db.commit().await?;
    Ok(())
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn neither_policy_can_expand_the_other() {
        for a in [Policy::Deny, Policy::Ask, Policy::Allow] {
            for b in [Policy::Deny, Policy::Ask, Policy::Allow] {
                let r = restrict(a, b);
                if a == Policy::Deny || b == Policy::Deny {
                    assert_eq!(r, Policy::Deny);
                } else if a == Policy::Ask || b == Policy::Ask {
                    assert_eq!(r, Policy::Ask);
                } else {
                    assert_eq!(r, Policy::Allow);
                }
            }
        }
    }
}
