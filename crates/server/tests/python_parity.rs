mod common;
#[tokio::test]
#[ignore = "optional cross-language migration validation; requires the baseline Python SDK test environment"]
async fn pinned_python_http_behavior_matches_rust() {
    let h = common::Harness::new().await;
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let python = std::env::var("AGENT_CONTROL_PARITY_PYTHON")
        .unwrap_or_else(|_| root.join(".venv/bin/python").to_string_lossy().into());
    let result = tokio::process::Command::new(&python)
        .arg(root.join("scripts/compare_python_reference.py"))
        .current_dir(root)
        .env("AGENT_CONTROL_RUST_TEST_ORIGIN", &h.origin)
        .output()
        .await
        .unwrap();
    assert!(
        result.status.success(),
        "{}",
        String::from_utf8_lossy(&result.stderr)
    );
    println!("{}", String::from_utf8_lossy(&result.stdout));
    let result = tokio::process::Command::new(python)
        .arg(
            std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
                .join("../../scripts/cross_sdk_rust.py"),
        )
        .env("AGENT_CONTROL_RUST_TEST_ORIGIN", &h.origin)
        .output()
        .await
        .unwrap();
    assert!(
        result.status.success(),
        "{}",
        String::from_utf8_lossy(&result.stderr)
    );
    println!("{}", String::from_utf8_lossy(&result.stdout));
}
