#![forbid(unsafe_code)]
#[tokio::main]
async fn main() {
    tracing_subscriber::fmt()
        .with_env_filter("agent_control_server=info,control_server=info")
        .with_target(false)
        .init();
    if let Err(error) = control_server::run(control_server::cli::parse(
        &std::env::args().skip(1).collect::<Vec<_>>(),
    ))
    .await
    {
        eprintln!("{error}");
        std::process::exit(1);
    }
}
