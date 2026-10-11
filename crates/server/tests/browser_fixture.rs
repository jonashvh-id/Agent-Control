mod common;
#[tokio::test]
#[ignore = "temporary manual browser QA server; never part of production runtime"]
async fn browser_fixture() {
    let h = common::Harness::new().await;
    h.policy("issues.create", "ask").await;
    let pending = h.execute(&h.proposal("browser-review-001")).await;
    assert_eq!(pending.0, 202);
    println!("Temporary isolated Rust UI: {}", h.origin);
    tokio::signal::ctrl_c().await.unwrap();
}
