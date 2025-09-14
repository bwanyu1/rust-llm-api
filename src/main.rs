mod api;
mod db;
mod groq;

use anyhow::{anyhow, Context, Result};
use reqwest::Client;
use std::net::SocketAddr;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};
use tower_http::services::{ServeDir, ServeFile};

#[tokio::main]
async fn main() -> Result<()> {
    // Logging
    tracing_subscriber::registry()
        .with(tracing_subscriber::EnvFilter::try_from_default_env().unwrap_or_else(|_| "info".into()))
        .with(tracing_subscriber::fmt::layer())
        .init();

    // Config
    let api_key = std::env::var("GROQ_API_KEY").map_err(|_| {
        anyhow!("GROQ_API_KEY is not set. Export it, e.g. `export GROQ_API_KEY=...`")
    })?;
    let model = std::env::var("GROQ_MODEL").unwrap_or_else(|_| "llama-3.3-70b-versatile".to_string());
    // Use a simple relative DB file by default to avoid path/permission issues
    let database_url = std::env::var("DATABASE_URL").unwrap_or_else(|_| "sqlite://app.db".to_string());

    // HTTP client
    let http = Client::builder().user_agent("rust-llm-api/0.1").build().context("failed to build HTTP client")?;

    // DB
    let db = db::Db::init(&database_url).await?;

    // API router
    let api_router = api::routes(api::AppState { db, http, groq_api_key: api_key, groq_model: model, database_url: database_url.clone() });

    // Static files under ./public with SPA-ish index fallback
    let static_service = ServeDir::new("public").not_found_service(ServeFile::new("public/index.html"));
    let app = axum::Router::new()
        .merge(api_router)
        .nest_service("/", static_service);

    // Bind
    let host = std::env::var("HOST").unwrap_or_else(|_| "0.0.0.0".to_string());
    let port: u16 = std::env::var("PORT").ok().and_then(|s| s.parse().ok()).unwrap_or(8080);
    let addr: SocketAddr = format!("{}:{}", host, port).parse().context("invalid HOST/PORT")?;
    tracing::info!("listening on http://{}", addr);
    axum::serve(tokio::net::TcpListener::bind(addr).await?, app).await?;
    Ok(())
}
