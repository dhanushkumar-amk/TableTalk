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
        .await
        .unwrap_or_else(|err| panic!("Server runtime encountered fatal error: {}", err));
}
