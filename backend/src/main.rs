use std::net::SocketAddr;
use tabletalk_backend::{build_router, Config};
use tracing::info;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt, EnvFilter};

#[tokio::main]
async fn main() {
    let config = Config::from_env();

    tracing_subscriber::registry()
        .with(
            EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new(&config.rust_log)),
        )
        .with(tracing_subscriber::fmt::layer())
        .init();

    let addr = SocketAddr::from(([0, 0, 0, 0], config.port));
    let listener = tokio::net::TcpListener::bind(addr)
        .await
        .unwrap_or_else(|err| panic!("Failed to bind TCP listener on {}: {}", addr, err));

    info!(
        "TableTalk backend listening on http://{} (MCP endpoint: http://{}/mcp)",
        addr, addr
    );

    let app = build_router(&config);

    axum::serve(listener, app)
        .with_graceful_shutdown(shutdown_signal())
        .await
        .unwrap_or_else(|err| panic!("Server runtime encountered fatal error: {}", err));

    info!("TableTalk server gracefully shut down");
}

async fn shutdown_signal() {
    let ctrl_c = async {
        tokio::signal::ctrl_c()
            .await
            .expect("failed to install Ctrl+C handler");
    };

    #[cfg(unix)]
    let terminate = async {
        match tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate()) {
            Ok(mut signal) => {
                signal.recv().await;
            }
            Err(e) => {
                tracing::warn!("Failed to install SIGTERM signal handler: {}", e);
                std::future::pending::<()>().await;
            }
        }
    };

    #[cfg(not(unix))]
    let terminate = std::future::pending::<()>();

    tokio::select! {
        _ = ctrl_c => {
            info!("Received SIGINT (Ctrl+C), initiating graceful shutdown");
        },
        _ = terminate => {
            info!("Received SIGTERM, initiating graceful shutdown");
        },
    }
}
