use crate::{
    App, approvals,
    error::{Error, Result},
    gateway::{Outcome, admission::Prepared, authorization, journal::Journal},
    models::{ExecutionState, Policy, Proposal},
    persistence::now,
};
use serde_json::{Value, json};
async fn mark(
    app: &App,
    token: &str,
    p: &Proposal,
    prepared: &Prepared,
    correlation: &str,
) -> Result<Policy> {
    if !crate::security::keys::equal(&app.db.keys.fingerprint(&p.binding()), &prepared.hash) {
        return Err(Error::new("approval_binding_mismatch", 403));
    }
    let mut db = app.db.write().await?;
    let (actor, mut mode) =
        authorization::check(app, &mut db, token, &p.connector, &prepared.tool).await?;
    if prepared.connector.requires_resources() {
        mode = crate::resources::authorize(
            &mut db,
            &actor.id,
            &p.connector,
            &Value::Object(p.parameters.clone()),
            mode,
        )
        .await?;
    }
    if mode == Policy::Ask && prepared.approval_id.is_empty() {
        return Err(Error::new("approval_required_after_policy_change", 403));
    }
    if !prepared.approval_id.is_empty() {
        approvals::revalidate(&mut db, &prepared.approval_id).await?;
    }
    if !crate::execution::dispatch(&mut db, &prepared.execution_id).await? {
        return Err(Error::new("execution_already_dispatched", 409));
    }
    journal(p, prepared, correlation)
        .event(&mut db, "execution.dispatched", mode.as_str(), None)
        .await?;
    db.commit().await?;
    Ok(mode)
}
fn journal<'a>(p: &'a Proposal, x: &'a Prepared, c: &'a str) -> Journal<'a> {
    Journal {
        known: p.connector == x.connector.id() && p.tool == x.tool.name,
        actor: Some(&x.actor),
        proposal: p,
        correlation: c,
        hash: &x.hash,
        execution_id: &x.execution_id,
        approval_id: &x.approval_id,
    }
}
pub async fn execute(
    app: &App,
    token: &str,
    p: &Proposal,
    c: &str,
    x: Prepared,
) -> Result<Outcome> {
    let _capacity = match app.provider_slots.clone().try_acquire_owned() {
        Ok(p) => p,
        Err(_) => return fail(app, p, c, &x, Error::new("connector_busy", 503), false).await,
    };
    let mode = match mark(app, token, p, &x, c).await {
        Ok(m) => m,
        Err(e) => return fail(app, p, c, &x, e, false).await,
    };
    // Durable dispatch record is committed. A cancelled task/crashed process must never authorize a retry.
    let parameters = Value::Object(p.parameters.clone());
    let result = tokio::time::timeout(
        std::time::Duration::from_secs(20),
        x.connector.execute(&p.tool, &parameters, &x.execution_id),
    )
    .await;
    let result = match result {
        Ok(Ok(value)) => value,
        Ok(Err(e)) => return fail(app, p, c, &x, e, true).await,
        Err(_) => {
            return fail(
                app,
                p,
                c,
                &x,
                Error::uncertain("connector_timeout", 504),
                true,
            )
            .await;
        }
    };
    let valid = jsonschema::validator_for(&x.tool.output_schema).is_ok_and(|v| v.is_valid(&result));
    if result.to_string().len() > 65536 || !valid {
        return fail(
            app,
            p,
            c,
            &x,
            Error::uncertain("invalid_connector_result", 502),
            true,
        )
        .await;
    }
    let receipt = x.connector.receipt(&result);
    let mut db = app.db.write().await?;
    let objects = if x.connector.requires_resources() {
        crate::resources::observe(
            &mut db,
            &app.db.keys,
            p.parameters["resource_id"].as_str().unwrap(),
            x.connector.observations(&result),
        )
        .await?
    } else {
        vec![]
    };
    crate::execution::finish(
        &mut db,
        &x.execution_id,
        ExecutionState::Executed,
        receipt.as_ref(),
    )
    .await?;
    sqlx::query("UPDATE agents SET last_used_at=? WHERE id=?")
        .bind(now())
        .bind(&x.actor.id)
        .execute(&mut *db)
        .await?;
    journal(p, &x, c)
        .event_objects(
            &mut db,
            "execution.executed",
            mode.as_str(),
            receipt.as_ref(),
            &objects,
        )
        .await?;
    db.commit().await?;
    Ok(Outcome {
        status: 200,
        body: json!({"status":"executed","execution_id":x.execution_id,"result":result,"receipt":receipt,"result_trust":"untrusted_connector_data","correlation_id":c}),
    })
}
async fn fail(
    app: &App,
    p: &Proposal,
    c: &str,
    x: &Prepared,
    e: Error,
    dispatched: bool,
) -> Result<Outcome> {
    let state = if dispatched && e.uncertain {
        ExecutionState::Uncertain
    } else {
        ExecutionState::Failed
    };
    let mut db = app.db.write().await?;
    crate::execution::finish(&mut db, &x.execution_id, state, None).await?;
    journal(p, x, c)
        .event(&mut db, e.code, "deny", None)
        .await?;
    db.commit().await?;
    Ok(Outcome {
        status: e.status,
        body: json!({"error":e.code,"execution_id":x.execution_id,"execution_state":state,"correlation_id":c}),
    })
}
