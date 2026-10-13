use std::net::SocketAddr;
use tabletalk_backend::{build_router_with_db, Config};
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

    let args: Vec<String> = std::env::args().collect();
    if args.len() > 1 && (args[1] == "seed" || args[1] == "init-db") {
        info!("Initializing database pool and running migrations...");
        let pool = tabletalk_backend::db::init_pool(&config.database_url)
            .await
            .unwrap_or_else(|err| panic!("Failed to initialize database: {}", err));

        info!("Seeding database with deterministic fictional data...");
        tabletalk_backend::db::seed_database(&pool, None)
            .await
            .unwrap_or_else(|err| panic!("Failed to seed database: {}", err));

        println!("==================================================");
        println!(" Database Seed Completed Successfully");
        println!(" Database URL: {}", config.database_url);
        println!("--------------------------------------------------");
        print_table_counts(&pool).await;
        println!("==================================================");
        return;
    }

    if args.len() > 1 && args[1] == "purge-expired" {
        info!("Initializing database pool and running migrations...");
        let pool = tabletalk_backend::db::init_pool(&config.database_url)
            .await
            .unwrap_or_else(|err| panic!("Failed to initialize database: {}", err));

        let days: i64 = if args.len() > 2 {
            args[2]
                .parse()
                .unwrap_or(tabletalk_backend::memory::DEFAULT_RETENTION_DAYS)
        } else {
            tabletalk_backend::memory::DEFAULT_RETENTION_DAYS
        };

        info!("Purging expired profiles older than {} days...", days);
        let clock = tabletalk_backend::availability::SystemClock;
        let purged = tabletalk_backend::memory::purge_expired_profiles(&pool, days, &clock)
            .await
            .unwrap_or_else(|err| panic!("Failed to purge expired profiles: {}", err));

        println!("==================================================");
        println!(" Expired Profiles Purge Completed");
        println!(" Purged Profiles Count: {}", purged);
        println!(" Retention Threshold: {} days", days);
        println!("==================================================");
        return;
    }

    info!("Initializing database pool and applying migrations...");
    let pool = tabletalk_backend::db::init_pool(&config.database_url)
        .await
        .unwrap_or_else(|err| panic!("Failed to initialize database: {}", err));

    if config.seed_on_start {
        let restaurant_count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM restaurant")
            .fetch_one(&pool)
            .await
            .unwrap_or(0);
        if restaurant_count == 0 {
            info!("SEED_ON_START is true and database is unseeded; seeding now...");
            tabletalk_backend::db::seed_database(&pool, None)
                .await
                .unwrap_or_else(|err| panic!("Failed to seed database on startup: {}", err));
        }
    }

    let addr = SocketAddr::from(([0, 0, 0, 0], config.port));
    let listener = tokio::net::TcpListener::bind(addr)
        .await
        .unwrap_or_else(|err| panic!("Failed to bind TCP listener on {}: {}", addr, err));

    info!(
        "TableTalk backend listening on http://{} (MCP endpoint: http://{}/mcp)",
        addr, addr
    );

    let app = build_router_with_db(&config, Some(pool));

    axum::serve(listener, app)
        .with_graceful_shutdown(shutdown_signal())
        .await
        .unwrap_or_else(|err| panic!("Server runtime encountered fatal error: {}", err));

    info!("TableTalk server gracefully shut down");
}

async fn print_table_counts(pool: &sqlx::SqlitePool) {
    let tables = [
        "restaurant",
        "opening_hours",
        "special_closures",
        "dining_tables",
        "menu_categories",
        "menu_items",
        "dietary_tags",
        "allergens",
        "menu_item_dietary_tags",
        "menu_item_allergens",
        "customers",
        "customer_preferences",
        "bookings",
        "orders",
        "order_items",
        "payments",
        "audit_events",
    ];

    println!("{:<28} | {:<8}", "Table Name", "Row Count");
    println!("{:-<28}-|-{:-<8}", "", "");
    for table in tables {
        let count_query = format!("SELECT COUNT(*) FROM {}", table);
        let count: i64 = sqlx::query_scalar(&count_query)
            .fetch_one(pool)
            .await
            .unwrap_or(-1);
        println!("{:<28} | {:<8}", table, count);
    }
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
