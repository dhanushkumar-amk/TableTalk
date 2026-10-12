use std::net::SocketAddr;
use tabletalk_backend::{build_router, Config};

#[tokio::test]
#[ignore = "manual server runner for live verification; run with cargo test --test server_runner -- --nocapture --ignored"]
async fn run_server_for_manual_verification() {
    let config = Config::from_env();
    let addr = SocketAddr::from(([127, 0, 0, 1], config.port));
    let listener = tokio::net::TcpListener::bind(addr)
        .await
        .unwrap_or_else(|e| panic!("Failed to bind to {}: {}", addr, e));

    println!("TableTalk MCP Server listening on http://{}", addr);

    let app = build_router(&config);
    axum::serve(listener, app).await.unwrap();
}
