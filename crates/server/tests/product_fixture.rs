mod common;
use control_core::{models::Policy, policy::Set, security::files};
#[tokio::test]
#[ignore = "manual product/model pilot; optional GitHub read requires explicit operator invocation"]
async fn product_fixture() {
    let live = std::env::var("AGENT_CONTROL_PILOT_GITHUB_LIVE").as_deref() == Ok("true");
    let native = std::env::var("AGENT_CONTROL_PILOT_GOOGLE_LIVE").as_deref() == Ok("true");
    let h = if native {
        let files = std::env::var("AGENT_CONTROL_GOOGLE_DRIVE_FILES")
            .expect("native pilot requires an explicit file allowlist")
            .split(',')
            .map(|id| id.trim().to_owned())
            .collect();
        let token_file = std::env::var("AGENT_CONTROL_GOOGLE_DRIVE_TOKEN_FILE")
            .expect("native pilot requires a private Google token file")
            .into();
        common::Harness::native_pilot(
            live,
            connector_google_drive::Config {
                files,
                token_file,
                timeout: std::time::Duration::from_secs(10),
            },
        )
        .await
    } else {
        common::Harness::pilot(live).await
    };
    h.policy("issues.list", "allow").await;
    for (tool, mode) in [("files.search", Policy::Allow), ("files.read", Policy::Ask)] {
        control_core::policy::set(
            &h.app,
            &h.actor,
            Set {
                agent_id: h.agent.clone(),
                connector: "google-drive".into(),
                tool: tool.into(),
                mode,
            },
        )
        .await
        .unwrap();
    }
    if !live {
        h.policy("issues.create", "ask").await;
    }
    if let Ok(path) = std::env::var("AGENT_CONTROL_PILOT_TOKEN_FILE") {
        let path = std::path::Path::new(&path);
        files::private_directory(path.parent().unwrap()).unwrap();
        files::create_private(path, h.token.as_bytes()).unwrap();
    }
    println!(
        "Product pilot: {} ; agent={} ; native Google Drive={native}; live GitHub read={live}",
        h.origin, h.agent
    );
    tokio::signal::ctrl_c().await.unwrap();
}
