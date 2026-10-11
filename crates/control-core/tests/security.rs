use control_core::{
    App,
    models::{Config, Policy, Proposal},
    persistence::Database,
    security::{files, keys::Keys, passwords},
};
use serde_json::json;
#[test]
fn preview_encryption_authenticated() {
    let keys = Keys::new(vec![7; 48]).unwrap();
    let input = json!({"title":"private title"});
    let a = keys.seal(&input).unwrap();
    let b = keys.seal(&input).unwrap();
    assert_ne!(a, b);
    assert!(!a.contains("private title"));
    assert_eq!(keys.open(&a).unwrap(), input);
    let wrong = Keys::new(vec![8; 48]).unwrap();
    assert!(wrong.open(&a).is_err());
    let mut changed = a.into_bytes();
    changed[20] = if changed[20] == b'A' { b'B' } else { b'A' };
    assert!(keys.open(std::str::from_utf8(&changed).unwrap()).is_err());
}
#[test]
fn fingerprints_bind_types_and_order() {
    let keys = Keys::new(vec![7; 48]).unwrap();
    assert_eq!(
        keys.fingerprint(&json!({"a":1,"b":2})),
        keys.fingerprint(&json!({"b":2,"a":1}))
    );
    assert_ne!(
        keys.fingerprint(&json!({"a":1})),
        keys.fingerprint(&json!({"a":"1"}))
    );
}
#[test]
fn password_hash_and_verification() {
    let hash = passwords::hash("Foundation9!Secure").unwrap();
    assert!(hash.starts_with("$argon2id$"));
    assert!(passwords::verify("Foundation9!Secure", &hash));
    assert!(!passwords::verify("wrong", &hash));
    assert!(!passwords::verify("password", "corrupt hash"));
}
#[test]
fn password_requirements() {
    for p in [
        "short",
        "nouppercase1234",
        "NOLOWERCASE1234",
        "MissingDigitsLong",
        "Has Whitespace9!Long",
    ] {
        assert!(passwords::hash(p).is_err());
    }
}
#[test]
fn malformed_proposals_and_overrides() {
    for p in [
        json!({"connector":"github","tool":"issues.create","parameters":{},"idempotency_key":"valid-key-001","actor_id":"override"}),
        json!({"connector":"github","tool":"issues.create","parameters":{},"idempotency_key":3}),
    ] {
        assert!(serde_json::from_value::<Proposal>(p).is_err());
    }
    let mut p:Proposal=serde_json::from_value(json!({"connector":"github","tool":"issues.create","parameters":{},"idempotency_key":"short"})).unwrap();
    assert!(p.validate().is_err());
    p.idempotency_key = "valid-key-001".into();
    p.parameters.insert("body".into(), json!("x".repeat(16385)));
    assert!(p.validate().is_err());
}
#[test]
fn deep_and_wide_parameters_rejected() {
    let mut p:Proposal=serde_json::from_value(json!({"connector":"github","tool":"issues.create","parameters":{},"idempotency_key":"valid-key-001"})).unwrap();
    let mut nested = json!(0);
    for _ in 0..34 {
        nested = json!([nested]);
    }
    p.parameters.insert("nested".into(), nested);
    assert!(p.validate().is_err());
    p.parameters.clear();
    p.parameters.insert("wide".into(), json!(vec![0; 1001]));
    assert!(p.validate().is_err());
}
#[test]
fn unknown_policy_rejected() {
    assert!(serde_json::from_value::<Policy>(json!("sometimes")).is_err());
}
#[tokio::test]
async fn independent_owned_runtime() {
    let temp = tempfile::tempdir().unwrap();
    let db = Database::open(&temp.path().join("runtime")).await.unwrap();
    assert_eq!(
        sqlx::query_scalar::<_, i64>("SELECT count(*) FROM humans")
            .fetch_one(&db.pool)
            .await
            .unwrap(),
        0
    );
}
#[tokio::test]
async fn refuses_unowned_python_runtime() {
    let temp = tempfile::tempdir().unwrap();
    let path = temp.path().join("runtime");
    std::fs::create_dir(&path).unwrap();
    std::fs::write(path.join(".agent-control"), "agent-control-v1\n").unwrap();
    assert!(Database::open(&path).await.is_err());
    assert_eq!(
        std::fs::read_to_string(path.join(".agent-control")).unwrap(),
        "agent-control-v1\n"
    );
}
#[tokio::test]
async fn runtime_exclusive_lock() {
    let temp = tempfile::tempdir().unwrap();
    let path = temp.path().join("runtime");
    let db = Database::open(&path).await.unwrap();
    assert!(Database::open(&path).await.is_err());
    db.pool.close().await;
    drop(db);
    assert!(Database::open(&path).await.is_ok());
}
#[tokio::test]
async fn crash_recovery_marks_unknown_and_preserves_keys() {
    let temp = tempfile::tempdir().unwrap();
    let config = Config::local(temp.path().join("runtime"));
    let app = App::new(config.clone(), vec![]).await.unwrap();
    let hash = app.db.keys.hash("test-token");
    let id = control_core::identity::create_admin(&app, "admin", "Foundation9!Secure", "admin")
        .await
        .unwrap();
    let agent = control_core::agents::create(
        &app,
        &control_core::models::Human {
            id,
            username: "admin".into(),
            role: "admin".into(),
            csrf: "".into(),
        },
        control_core::agents::Create {
            name: "Crash test".into(),
            description: "".into(),
            provider: "".into(),
            runtime: "".into(),
            scopes: vec![],
        },
    )
    .await
    .unwrap();
    for (id, status) in [
        ("prepared", "prepared"),
        ("dispatched", "dispatched"),
        ("executed", "executed"),
    ] {
        sqlx::query("INSERT INTO executions(id,agent_id,idempotency_key,request_hash,status,created_at) VALUES(?,?,?,?,?,?)").bind(id).bind(agent["id"].as_str().unwrap()).bind(id).bind("hash").bind(status).bind("time").execute(&app.db.pool).await.unwrap();
    }
    app.db.pool.close().await;
    drop(app);
    let app = App::new(config, vec![]).await.unwrap();
    assert_eq!(hash, app.db.keys.hash("test-token"));
    let states: Vec<(String, String)> =
        sqlx::query_as("SELECT id,status FROM executions ORDER BY id")
            .fetch_all(&app.db.pool)
            .await
            .unwrap();
    assert_eq!(
        states,
        vec![
            ("dispatched".into(), "uncertain".into()),
            ("executed".into(), "executed".into()),
            ("prepared".into(), "uncertain".into())
        ]
    );
}
#[cfg(unix)]
#[tokio::test]
async fn symlink_runtime_refused() {
    let temp = tempfile::tempdir().unwrap();
    let actual = temp.path().join("actual");
    std::fs::create_dir(&actual).unwrap();
    let link = temp.path().join("linked");
    std::os::unix::fs::symlink(&actual, &link).unwrap();
    assert!(Database::open(&link).await.is_err());
}
#[test]
fn audit_summary_omits_values() {
    let v = control_core::audit::summarize(&json!({"password":"never-log-me","count":1}));
    assert_eq!(v["parameter_count"], 2);
    assert!(!v.to_string().contains("never-log-me"));
}
#[cfg(unix)]
#[test]
fn private_file_reads_bounded() {
    let temp = tempfile::tempdir().unwrap();
    let path = temp.path().join("secret");
    files::create_private(&path, b"123456").unwrap();
    assert!(files::private_read(&path, 5).is_err());
    assert_eq!(files::private_read(&path, 6).unwrap(), b"123456");
}
