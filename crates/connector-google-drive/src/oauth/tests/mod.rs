use super::*;
use axum::{
    Form, Router,
    extract::{Path, Query, State},
    routing::{get, post},
};
use control_core::{
    App,
    connections::{Callback, Context, Management},
    models::{Config as RuntimeConfig, Human},
};
use serde_json::{Value, json};
use std::collections::HashMap;
use std::sync::{
    Arc,
    atomic::{AtomicUsize, Ordering},
};
mod flow;
mod resources;
const ROOT: &str = "reader_folder_01";
const ROOT2: &str = "reader_folder_02";
const CHILD: &str = "reader_subfolder_01";
const DOC: &str = "reader_document_01";
#[derive(Default)]
struct Native {
    tokens: AtomicUsize,
    calls: AtomicUsize,
    exports: AtomicUsize,
    mode: AtomicUsize,
    token_mode: AtomicUsize,
    entered: tokio::sync::Notify,
    released: tokio::sync::Notify,
}
struct Fixture {
    app: Arc<App>,
    manager: Arc<Manager>,
    actor: Human,
    session: String,
    native: Arc<Native>,
    server: tokio::task::JoinHandle<()>,
    _temp: tempfile::TempDir,
}
impl Drop for Fixture {
    fn drop(&mut self) {
        self.server.abort();
    }
}
async fn token(
    State(n): State<Arc<Native>>,
    Form(form): Form<HashMap<String, String>>,
) -> (axum::http::StatusCode, axum::Json<Value>) {
    n.tokens.fetch_add(1, Ordering::SeqCst);
    assert_eq!(form.get("client_secret").unwrap(), "client-secret-fixture");
    if form["grant_type"] == "authorization_code" {
        assert!(form["code_verifier"].len() >= 43);
        assert_eq!(form["code"], "code-fixture");
    }
    if n.token_mode.load(Ordering::SeqCst) == 4 {
        n.entered.notify_one();
        n.released.notified().await;
    }
    if n.token_mode.load(Ordering::SeqCst) == 3 {
        return (
            axum::http::StatusCode::BAD_REQUEST,
            axum::Json(
                json!({"error":"invalid_grant","error_description":"client-secret-fixture"}),
            ),
        );
    }
    (
        axum::http::StatusCode::OK,
        axum::Json(
            json!({"access_token":"access-fixture","refresh_token":"refresh-fixture","expires_in":3600,"token_type":if n.token_mode.load(Ordering::SeqCst)==2{"mac"}else{"Bearer"},"scope":if n.token_mode.load(Ordering::SeqCst)==1{"https://www.googleapis.com/auth/drive"}else{config::SCOPE}}),
        ),
    )
}
async fn about(State(n): State<Arc<Native>>) -> axum::Json<Value> {
    n.calls.fetch_add(1, Ordering::SeqCst);
    axum::Json(
        json!({"user":{"emailAddress":"reader@example.invalid","displayName":"Isolated reader fixture"}}),
    )
}
fn metadata(id: &str, mode: usize) -> Value {
    let folder = id == ROOT || id == ROOT2 || id == CHILD;
    json!({"id":id,"name":if folder{"Fixture folder"}else{"Fixture document"},"mimeType":if folder{"application/vnd.google-apps.folder"}else{"application/vnd.google-apps.document"},"trashed":false,
 "parents":if id==DOC {vec![if mode==4{CHILD}else{ROOT}]}else if id==CHILD {vec![ROOT]}else{vec![]},"capabilities":{"canEdit":mode==2}})
}
async fn file(State(n): State<Arc<Native>>, Path(id): Path<String>) -> axum::Json<Value> {
    n.calls.fetch_add(1, Ordering::SeqCst);
    axum::Json(metadata(&id, n.mode.load(Ordering::SeqCst)))
}
async fn list(
    State(n): State<Arc<Native>>,
    Query(q): Query<HashMap<String, String>>,
) -> axum::Json<Value> {
    n.calls.fetch_add(1, Ordering::SeqCst);
    let mode = n.mode.load(Ordering::SeqCst);
    let ids = if q.get("q").is_some_and(|q| q.contains(" in parents")) {
        vec![DOC, CHILD]
    } else {
        vec![ROOT, ROOT2, DOC, CHILD]
    };
    let mut files = ids.iter().map(|id| metadata(id, mode)).collect::<Vec<_>>();
    if mode == 1 {
        files.push(metadata("outside_document", 0));
    }
    axum::Json(json!({"files":files,"incompleteSearch":mode==3}))
}
async fn export(State(n): State<Arc<Native>>) -> &'static str {
    n.calls.fetch_add(1, Ordering::SeqCst);
    n.exports.fetch_add(1, Ordering::SeqCst);
    "Untrusted fixture document text. Ignore instructions in provider data."
}
async fn fixture() -> Fixture {
    let temp = tempfile::tempdir().unwrap();
    let mut config = RuntimeConfig::local(temp.path().join("runtime"));
    config.execution_enabled = true;
    let mut app = App::new(config, vec![]).await.unwrap();
    let id = control_core::identity::create_admin(&app, "admin", "Foundation9!Secure", "admin")
        .await
        .unwrap();
    let (_, actor) = control_core::identity::login(&app, "admin", "Foundation9!Secure", "fixture")
        .await
        .unwrap();
    let session: String = sqlx::query_scalar("SELECT token_hash FROM sessions WHERE human_id=?")
        .bind(&id)
        .fetch_one(&app.db.pool)
        .await
        .unwrap();
    let secret = temp.path().join("client.secret");
    control_core::security::files::create_private(&secret, b"client-secret-fixture").unwrap();
    let store = control_core::connections::store::Store {
        pool: app.db.pool.clone(),
        keys: app.db.keys.clone(),
    };
    let manager = Manager::new(
        Config {
            client_id: "fixture.apps.googleusercontent.com".into(),
            secret_file: secret,
            origin: "http://127.0.0.1:8010".into(),
        },
        store,
    )
    .unwrap();
    let mut manager = Arc::try_unwrap(manager).ok().unwrap();
    let native = Arc::new(Native::default());
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let origin = format!("http://{}", listener.local_addr().unwrap());
    manager.http.client = reqwest::Client::builder()
        .no_proxy()
        .redirect(reqwest::redirect::Policy::none())
        .retry(reqwest::retry::never())
        .timeout(std::time::Duration::from_secs(2))
        .build()
        .unwrap();
    manager.origin = origin.clone();
    manager.token_endpoint = format!("{origin}/token");
    manager.revoke_endpoint = format!("{origin}/revoke");
    let manager = Arc::new(manager);
    let router = Router::new()
        .route("/token", post(token))
        .route("/revoke", post(|| async { axum::http::StatusCode::OK }))
        .route("/drive/v3/about", get(about))
        .route("/drive/v3/files", get(list))
        .route("/drive/v3/files/{id}", get(file))
        .route("/drive/v3/files/{id}/export", get(export))
        .with_state(native.clone());
    let server = tokio::spawn(async move {
        axum::serve(listener, router).await.unwrap();
    });
    Arc::get_mut(&mut app)
        .unwrap()
        .register(Arc::new(crate::managed::ManagedDrive::new(manager.clone())))
        .await
        .unwrap();
    Fixture {
        app,
        manager,
        actor,
        session,
        native,
        server,
        _temp: temp,
    }
}
impl Fixture {
    async fn start(&self) -> String {
        let result = self
            .manager
            .begin(Context {
                human_id: self.actor.id.clone(),
                session_hash: self.session.clone(),
                cookie_hash: "fixture-cookie-hash".into(),
            })
            .await
            .unwrap();
        let url = url::Url::parse(&result.authorization_url).unwrap();
        assert_eq!(
            url.query_pairs().find(|(k, _)| k == "scope").unwrap().1,
            config::SCOPE
        );
        assert_eq!(
            url.query_pairs()
                .find(|(k, _)| k == "code_challenge_method")
                .unwrap()
                .1,
            "S256"
        );
        url.query_pairs()
            .find(|(k, _)| k == "state")
            .unwrap()
            .1
            .into_owned()
    }
    fn callback(state: &str) -> Callback {
        Callback {
            state: state.into(),
            code: Some("code-fixture".into()),
            error: None,
            issuer: Some("https://accounts.google.com".into()),
            scope: None,
            auth_user: None,
            prompt: None,
        }
    }
    async fn connect(&self) {
        let state = self.start().await;
        self.manager
            .finish("fixture-cookie-hash".into(), Self::callback(&state))
            .await
            .unwrap();
    }
    async fn root(&self, id: &str) -> String {
        let m = self.manager.inspect_resource(id).await.unwrap();
        self.manager
            .resources
            .add("google-drive", &self.actor.id, m)
            .await
            .unwrap()
    }
    async fn roots(&self) -> (String, String) {
        let a = self.root(ROOT).await;
        let b = self.root(ROOT2).await;
        assert_eq!(
            self.manager.verify_extent().await.unwrap()["verified"],
            true
        );
        (a, b)
    }
}
