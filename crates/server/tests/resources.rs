mod common;
use common::Harness;
use reqwest::Method;
use serde_json::{Value, json};
#[tokio::test]
async fn management_and_operations_routes_require_a_current_admin_not_an_agent() {
    let h = Harness::new().await;
    for path in [
        "/api/v1/connections/google-drive",
        "/api/v1/resources",
        "/api/v1/resource-policies",
        "/api/v1/operations",
        "/api/v1/connections/google-drive/folders",
    ] {
        assert_eq!(
            h.agent_request(Method::GET, path)
                .send()
                .await
                .unwrap()
                .status(),
            401
        );
        assert_eq!(
            h.client
                .get(format!("{}{path}", h.origin))
                .send()
                .await
                .unwrap()
                .status(),
            401
        );
    }
    for (method, path, body) in [
        (
            Method::POST,
            "/api/v1/connections/google-drive/start",
            json!({}),
        ),
        (
            Method::POST,
            "/api/v1/resources",
            json!({"connector":"google-drive","provider_id":"reader_folder_01"}),
        ),
        (
            Method::PUT,
            "/api/v1/resource-policies",
            json!({"agent_id":h.agent,"resource_id":"00000000000000000000000000000000","mode":"allow"}),
        ),
    ] {
        assert_eq!(
            h.agent_request(method.clone(), path)
                .json(&body)
                .send()
                .await
                .unwrap()
                .status(),
            401
        );
        assert_eq!(
            h.client
                .request(method, format!("{}{path}", h.origin))
                .header("cookie", &h.cookie)
                .json(&body)
                .send()
                .await
                .unwrap()
                .status(),
            403
        );
    }
    let login = h
        .client
        .post(format!("{}/api/v1/auth/login", h.origin))
        .header("x-control-request", "1")
        .json(&json!({"username":"reviewer","password":"Foundation9!Review"}))
        .send()
        .await
        .unwrap();
    let cookie = login.headers()["set-cookie"]
        .to_str()
        .unwrap()
        .split(';')
        .next()
        .unwrap()
        .to_string();
    for path in [
        "/api/v1/resources",
        "/api/v1/operations",
        "/api/v1/connections/google-drive",
    ] {
        assert_eq!(
            h.client
                .get(format!("{}{path}", h.origin))
                .header("cookie", &cookie)
                .send()
                .await
                .unwrap()
                .status(),
            403
        );
    }
    assert_eq!(h.calls(), 0);
}
#[tokio::test]
async fn no_configuration_is_reported_honestly_and_cannot_enable_resource_execution() {
    let h = Harness::new().await;
    let status: Value = h
        .human(Method::GET, "/api/v1/connections/google-drive")
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    assert_eq!(status["state"], "oauth_not_configured");
    assert_eq!(status["automatic_refresh"], false);
    assert_eq!(
        status["redirect_uri"],
        format!("{}/api/v1/connections/google-drive/callback", h.origin)
    );
    assert_eq!(
        h.human(Method::POST, "/api/v1/connections/google-drive/start")
            .json(&json!({}))
            .send()
            .await
            .unwrap()
            .status(),
        503
    );
    assert_eq!(h.human(Method::PUT,"/api/v1/resource-policies").json(&json!({"agent_id":h.agent,"resource_id":"00000000000000000000000000000000","mode":"allow"})).send().await.unwrap().status(),404);
    let operations: Value = h
        .human(Method::GET, "/api/v1/operations")
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    assert_eq!(operations["agents"][0]["active_credentials"], 1);
    assert!(operations["agents"][0]["last_request_at"].is_null());
    assert_eq!(operations["metrics"]["completed_24h"], 0);
    assert!(operations.to_string().to_lowercase().contains("stateless"));
}
#[tokio::test]
async fn integrated_oauth_uses_a_bound_lax_cookie_without_exposing_native_tokens() {
    let temp = tempfile::tempdir().unwrap();
    let secret = temp.path().join("client.secret");
    std::fs::write(&secret, "client-secret-fixture").unwrap();
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(&secret, std::fs::Permissions::from_mode(0o600)).unwrap();
    }
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let origin = format!("http://{}", listener.local_addr().unwrap());
    let mut config = control_core::models::Config::local(temp.path().join("runtime"));
    config.origin = origin.clone();
    let oauth = connector_google_drive::oauth::Config {
        client_id: "fixture.apps.googleusercontent.com".into(),
        secret_file: secret,
        origin: origin.clone(),
    };
    let app = control_server::compose_with_oauth(config, None, None, Some(oauth))
        .await
        .unwrap();
    control_core::identity::create_admin(&app, "admin", "Foundation9!Secure", "admin")
        .await
        .unwrap();
    let router = control_server::router(app.clone());
    let server = tokio::spawn(async move {
        axum::serve(
            listener,
            router.into_make_service_with_connect_info::<std::net::SocketAddr>(),
        )
        .await
        .unwrap();
    });
    let client = reqwest::Client::builder()
        .no_proxy()
        .redirect(reqwest::redirect::Policy::none())
        .build()
        .unwrap();
    let login = client
        .post(format!("{origin}/api/v1/auth/login"))
        .header("x-control-request", "1")
        .json(&json!({"username":"admin","password":"Foundation9!Secure"}))
        .send()
        .await
        .unwrap();
    let session = login.headers()["set-cookie"]
        .to_str()
        .unwrap()
        .split(';')
        .next()
        .unwrap()
        .to_string();
    let actor: Value = login.json().await.unwrap();
    let start = client
        .post(format!("{origin}/api/v1/connections/google-drive/start"))
        .header("cookie", session)
        .header("x-control-request", "1")
        .header("x-csrf-token", actor["csrf"].as_str().unwrap())
        .send()
        .await
        .unwrap();
    assert_eq!(start.status(), 200);
    let cookie = start.headers()["set-cookie"].to_str().unwrap().to_string();
    assert!(cookie.contains("HttpOnly; SameSite=Lax"));
    assert!(cookie.contains("Path=/api/v1/connections/google-drive/callback"));
    let payload: Value = start.json().await.unwrap();
    let url = url::Url::parse(payload["authorization_url"].as_str().unwrap()).unwrap();
    let params = url
        .query_pairs()
        .collect::<std::collections::HashMap<_, _>>();
    assert_eq!(
        params["scope"],
        "https://www.googleapis.com/auth/drive.readonly"
    );
    assert_eq!(params["code_challenge_method"], "S256");
    let callback = format!(
        "{origin}/api/v1/connections/google-drive/callback?state={}&code=",
        params["state"]
    );
    assert_eq!(client.get(&callback).send().await.unwrap().status(), 403);
    // An empty code is rejected before external I/O. The cross-site callback needs the bound flow cookie, not the Strict human cookie.
    let reply = client
        .get(&callback)
        .header("cookie", cookie.split(';').next().unwrap())
        .send()
        .await
        .unwrap();
    assert_eq!(reply.status(), 303);
    assert_eq!(
        reply.headers()["location"],
        "/?google=connection_failed#integrations"
    );
    assert!(
        reply.headers()["set-cookie"]
            .to_str()
            .unwrap()
            .contains("Max-Age=0")
    );
    let consumed: i64 = sqlx::query_scalar("SELECT consumed FROM oauth_attempts")
        .fetch_one(&app.db.pool)
        .await
        .unwrap();
    assert_eq!(consumed, 1);
    let events = control_core::audit::list(&app.db.pool)
        .await
        .unwrap()
        .to_string();
    assert!(!events.contains("client-secret-fixture"));
    assert!(!events.contains(params["state"].as_ref()));
    server.abort();
}
