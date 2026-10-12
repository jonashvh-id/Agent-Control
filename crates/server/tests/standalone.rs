use reqwest::Client;
#[tokio::test]
async fn copied_executable_starts_without_python_or_workspace() {
    let temp = tempfile::tempdir().unwrap();
    let binary = temp.path().join("agent-control-server");
    std::fs::copy(env!("CARGO_BIN_EXE_agent-control-server"), &binary).unwrap();
    let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
    let port = listener.local_addr().unwrap().port();
    drop(listener);
    let mut command = tokio::process::Command::new(binary);
    command.env_clear();
    if let Ok(profile) = std::env::var("LLVM_PROFILE_FILE") {
        command.env("LLVM_PROFILE_FILE", profile);
    }
    let mut child = command
        .args(["serve", "--port", &port.to_string()])
        .env("PATH", "")
        .env("AGENT_CONTROL_DATA_DIR", temp.path().join("runtime"))
        .current_dir(temp.path())
        .kill_on_drop(true)
        .spawn()
        .unwrap();
    let client = Client::builder().no_proxy().build().unwrap();
    let origin = format!("http://127.0.0.1:{port}");
    tokio::time::timeout(std::time::Duration::from_secs(10), async {
        loop {
            if client
                .get(format!("{origin}/health"))
                .send()
                .await
                .is_ok_and(|r| r.status() == 200)
            {
                break;
            }
            assert!(child.try_wait().unwrap().is_none());
            tokio::time::sleep(std::time::Duration::from_millis(50)).await;
        }
    })
    .await
    .unwrap();
    for path in ["/", "/ui/src/app.js", "/ui/favicon.svg"] {
        assert_eq!(
            client
                .get(format!("{origin}{path}"))
                .send()
                .await
                .unwrap()
                .status(),
            200
        );
    }
    assert_eq!(
        client
            .get(format!("{origin}/api/v1/auth/me"))
            .send()
            .await
            .unwrap()
            .status(),
        401
    );
    assert_eq!(
        client
            .get(format!("{origin}/mcp/"))
            .send()
            .await
            .unwrap()
            .status(),
        401
    );
    child.kill().await.unwrap();
    assert!(temp.path().join("runtime/.agent-control-rust").exists());
}
#[tokio::test]
async fn help_does_not_create_runtime() {
    let temp = tempfile::tempdir().unwrap();
    let data = temp.path().join("runtime");
    let result = tokio::process::Command::new(env!("CARGO_BIN_EXE_agent-control-server"))
        .arg("--help")
        .env_clear()
        .env("PATH", "")
        .env("AGENT_CONTROL_DATA_DIR", &data)
        .output()
        .await
        .unwrap();
    assert!(result.status.success());
    assert!(String::from_utf8_lossy(&result.stdout).contains("create-admin"));
    assert!(!data.exists());
}
