mod api;
mod db;

use dotenv::dotenv;
use std::env;
use anyhow::{Context, Result};
use std::net::SocketAddr;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};
use tower_http::services::{ServeDir, ServeFile};
use tower_http::cors::{Any, CorsLayer};
use axum::http::Method;

#[tokio::main]
async fn main() -> Result<()> {
    // Logging
    tracing_subscriber::registry()
        .with(tracing_subscriber::EnvFilter::try_from_default_env().unwrap_or_else(|_| "info".into()))
        .with(tracing_subscriber::fmt::layer())
        .init();
    dotenv().ok();
    // Use a simple relative DB file by default to avoid path/permission issues
    let database_url = env::var("DATABASE_URL").unwrap_or_else(|_| "sqlite://app.db".to_string());

    // DB
    let db = db::Db::init(&database_url).await?;

    // API router
    let api_router = api::routes(api::AppState { db, database_url: database_url.clone() });

    // Static files under ./public with SPA-ish index fallback
    let static_service = ServeDir::new("public").not_found_service(ServeFile::new("public/index.html"));
    // CORS (dev use: allow any origin/method/header)
    let cors = CorsLayer::new()
        .allow_origin(Any)
        .allow_methods([Method::GET, Method::POST, Method::DELETE, Method::PATCH, Method::OPTIONS])
        .allow_headers(Any);

    let app = axum::Router::new()
        .merge(api_router)
        .nest_service("/", static_service)
        .layer(cors);

    // Bind
    let host = env::var("HOST").unwrap_or_else(|_| "0.0.0.0".to_string());
    let port: u16 = env::var("PORT").ok().and_then(|s| s.parse().ok()).unwrap_or(8080);
    let addr: SocketAddr = format!("{}:{}", host, port).parse().context("invalid HOST/PORT")?;
    tracing::info!("listening on http://{}", addr);
    axum::serve(tokio::net::TcpListener::bind(addr).await?, app).await?;
    Ok(())
}
