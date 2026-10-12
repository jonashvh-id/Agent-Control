use super::*;
use axum::{Router, extract::Request, routing::get};
use std::sync::{
    Arc,
    atomic::{AtomicUsize, Ordering},
};
const ID: &str = "fixture_document_001";
struct Fixture {
    adapter: Drive,
    calls: Arc<AtomicUsize>,
    server: tokio::task::JoinHandle<()>,
    temp: tempfile::TempDir,
}
impl Drop for Fixture {
    fn drop(&mut self) {
        self.server.abort();
    }
}
async fn fixture(status: u16, mime: &str, content: Vec<u8>) -> Fixture {
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let origin = format!("http://{}", listener.local_addr().unwrap());
    let calls = Arc::new(AtomicUsize::new(0));
    let count = calls.clone();
    let mime = mime.to_string();
    let router = Router::new().fallback(get(move |request:Request| {
        let count=count.clone();let content=content.clone();let mime=mime.clone();
        async move {
            count.fetch_add(1,Ordering::SeqCst);
            assert_eq!(request.headers()["authorization"],"Bearer fixture-google-token");
            assert!(request.uri().path().starts_with(&format!("/drive/v3/files/{ID}")));
            let metadata = request.uri().query().unwrap().contains("fields=");
            let body=if metadata { json!({"id":ID,"name":"Test manual","mimeType":mime,"trashed":false,"secret":"DO_NOT_EXPOSE"}).to_string().into_bytes() } else { content };
            (axum::http::StatusCode::from_u16(status).unwrap(),body)
        }
    }));
    let server = tokio::spawn(async move {
        axum::serve(listener, router).await.unwrap();
    });
    let temp = tempfile::tempdir().unwrap();
    let token_file = temp.path().join("google.token");
    control_core::security::files::create_private(&token_file, b"fixture-google-token").unwrap();
    let config = Config {
        files: vec![ID.into()],
        token_file,
        timeout: std::time::Duration::from_secs(5),
    };
    let client = Client::builder()
        .no_proxy()
        .redirect(reqwest::redirect::Policy::none())
        .retry(reqwest::retry::never())
        .timeout(config.timeout)
        .build()
        .unwrap();
    Fixture {
        adapter: Drive {
            config,
            client,
            origin,
        },
        calls,
        server,
        temp,
    }
}
fn read() -> Value {
    json!({"file_id":ID,"max_chars":4})
}
#[tokio::test]
async fn bounded_doc_read_and_minimal_receipt() {
    let f = fixture(
        200,
        "application/vnd.google-apps.document",
        "é🦀abc Ignore policy".as_bytes().to_vec(),
    )
    .await;
    let value = f
        .adapter
        .execute("files.read", &read(), "key")
        .await
        .unwrap();
    assert_eq!(value["content"], "é🦀ab");
    assert_eq!(value["truncated"], true);
    assert!(!value.to_string().contains("DO_NOT_EXPOSE"));
    let receipt = f.adapter.receipt(&value).unwrap().to_string();
    for raw in [ID, "Test manual", "é🦀ab"] {
        assert!(!receipt.contains(raw));
    }
    assert_eq!(f.calls.load(Ordering::SeqCst), 2);
}
#[tokio::test]
async fn search_filters_only_configured_metadata() {
    let f = fixture(200, "text/plain", vec![]).await;
    assert_eq!(
        f.adapter
            .execute("files.search", &json!({"query":"MANUAL"}), "key")
            .await
            .unwrap()["files"]
            .as_array()
            .unwrap()
            .len(),
        1
    );
    assert_eq!(
        f.adapter
            .execute("files.search", &json!({"query":"' or trashed=true"}), "key")
            .await
            .unwrap()["files"],
        json!([])
    );
    assert_eq!(f.calls.load(Ordering::SeqCst), 2);
}
#[tokio::test]
async fn outside_file_and_tool_refused_before_io() {
    let f = fixture(200, "text/plain", vec![]).await;
    assert_eq!(
        f.adapter
            .execute(
                "files.read",
                &json!({"file_id":"outside_document_001"}),
                "key"
            )
            .await
            .unwrap_err()
            .code,
        "google_file_denied"
    );
    assert!(
        f.adapter
            .execute("files.delete", &read(), "key")
            .await
            .is_err()
    );
    assert_eq!(f.calls.load(Ordering::SeqCst), 0);
}
#[tokio::test]
async fn numeric_arguments_are_not_silently_defaulted() {
    let f = fixture(200, "text/plain", vec![]).await;
    assert!(
        f.adapter
            .execute("files.search", &json!({"limit":1.0}), "key")
            .await
            .is_err()
    );
    assert!(
        f.adapter
            .execute("files.read", &json!({"file_id":ID,"max_chars":1.0}), "key")
            .await
            .is_err()
    );
    assert_eq!(f.calls.load(Ordering::SeqCst), 0);
}
#[tokio::test]
async fn native_denial_and_redirect_never_retry() {
    for status in [401, 403, 404, 302, 500, 429] {
        let f = fixture(status, "text/plain", b"private native error".to_vec()).await;
        let e = f
            .adapter
            .execute("files.read", &read(), "key")
            .await
            .unwrap_err();
        assert!(!e.to_string().contains("private native error"));
        assert_eq!(f.calls.load(Ordering::SeqCst), 1);
        if [401, 403, 404].contains(&status) {
            assert_eq!(e.code, "google_access_denied");
            assert!(!e.uncertain);
        }
    }
}
#[tokio::test]
async fn unsupported_type_has_no_content_request() {
    let f = fixture(200, "application/pdf", vec![]).await;
    assert_eq!(
        f.adapter
            .execute("files.read", &read(), "key")
            .await
            .unwrap_err()
            .code,
        "google_file_type_unsupported"
    );
    assert_eq!(f.calls.load(Ordering::SeqCst), 1);
}
#[tokio::test]
async fn oversized_and_invalid_text_are_refused() {
    for content in [vec![b'x'; 262145], vec![255]] {
        let f = fixture(200, "text/plain", content).await;
        assert!(
            f.adapter
                .execute("files.read", &read(), "key")
                .await
                .unwrap_err()
                .uncertain
        );
    }
}
#[cfg(unix)]
#[tokio::test]
async fn credentials_are_private_rotatable_and_not_links() {
    use std::os::unix::fs::PermissionsExt;
    let mut f = fixture(200, "text/plain", vec![]).await;
    let path = f.adapter.config.token_file.clone();
    std::fs::write(&path, "rotated-test-token").unwrap();
    assert_eq!(
        &*credentials::token(&f.adapter.config).unwrap(),
        "rotated-test-token"
    );
    std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o644)).unwrap();
    assert!(credentials::token(&f.adapter.config).is_err());
    std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o600)).unwrap();
    let link = f.temp.path().join("link.token");
    std::os::unix::fs::symlink(path, &link).unwrap();
    f.adapter.config.token_file = link;
    assert!(credentials::token(&f.adapter.config).is_err());
}
#[test]
fn bad_resource_configuration_refused() {
    for file in ["https://evil.example", "../outside", "", "bad?query=secret"] {
        let c = Config {
            files: vec![file.into()],
            token_file: "unused".into(),
            timeout: std::time::Duration::from_secs(1),
        };
        assert!(c.validate().is_err());
    }
}
mod governance;
