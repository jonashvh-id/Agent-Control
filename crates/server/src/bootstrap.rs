use crate::cli::Command;
use control_core::{
    error::{Error, Result},
    security::keys::equal,
};
pub async fn run(command: Result<Command>) -> Result<()> {
    let command = command?;
    if command == Command::Help {
        println!(
            "Agent Control Rust\nUsage: agent-control-server serve [--port PORT]\n       agent-control-server create-admin [--username NAME]\nConfiguration: AGENT_CONTROL_* environment variables. Use an empty dedicated Rust data directory."
        );
        return Ok(());
    }
    let (mut config, github) = crate::config::from_env()?;
    if let Command::Serve { port: Some(port) } = command {
        let mut origin = crate::config::validate_origin(&config)?;
        origin
            .set_port(Some(port))
            .map_err(|_| Error::new("invalid_port", 400))?;
        config.origin = origin.as_str().trim_end_matches('/').into();
    }
    let app = crate::compose(config, github).await?;
    if let Command::CreateAdmin { username } = command {
        let password =
            zeroize::Zeroizing::new(rpassword::prompt_password("New administrator password: ")?);
        let repeated = zeroize::Zeroizing::new(rpassword::prompt_password("Repeat password: ")?);
        if !equal(&password, &repeated) {
            return Err(Error::new("passwords_differ", 400));
        }
        control_core::identity::create_admin(&app, &username, &password, "admin").await?;
        println!("Administrator created in the Agent Control Rust data directory.");
        return Ok(());
    }
    let url = crate::config::validate_origin(&app.config)?;
    let port = url.port_or_known_default().unwrap_or(8010);
    let listener = tokio::net::TcpListener::bind(("127.0.0.1", port)).await?;
    tracing::info!(port, "Agent Control Rust server started on loopback");
    axum::serve(
        listener,
        crate::router(app).into_make_service_with_connect_info::<std::net::SocketAddr>(),
    )
    .with_graceful_shutdown(async {
        let _ = tokio::signal::ctrl_c().await;
    })
    .await?;
    Ok(())
}
