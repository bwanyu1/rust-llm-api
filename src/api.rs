use crate::db::{self, Db, Summary, SummaryListItem};
use crate::groq;
use anyhow::Result;
use axum::{
    body::Bytes,
    extract::{Path, State},
    http::{header, HeaderMap},
    routing::{get, post},
    Json, Router,
};
use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::sync::Arc;

#[derive(Clone)]
pub struct AppState {
    pub db: Db,
    pub http: Client,
    pub groq_api_key: String,
    pub groq_model: String,
    pub database_url: String,
}

#[derive(Deserialize)]
pub struct SummarizeRequest { pub text: String }

#[derive(Serialize)]
pub struct SummarizeResponse {
    pub id: i64,
    pub summary: String,
}

#[derive(Serialize)]
pub struct ListResponse {
    pub items: Vec<ListItem>,
}

#[derive(Serialize)]
pub struct ListItem {
    pub id: i64,
    pub created_at: String,
    pub summary_preview: String,
}

#[derive(Serialize)]
pub struct DetailResponse {
    pub item: Summary,
}

pub fn routes(state: AppState) -> Router {
    Router::new()
        .route("/api/summarize", post(summarize))
        .route("/api/summaries", get(list))
        .route("/api/summaries/:id", get(detail))
        .route("/api/debug", get(debug))
        .with_state(Arc::new(state))
}

async fn summarize(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    body: Bytes,
) -> Result<Json<SummarizeResponse>, (axum::http::StatusCode, String)> {
    // Accept either application/json {"text": "..."} or text/plain raw body
    let ct = headers
        .get(header::CONTENT_TYPE)
        .and_then(|v| v.to_str().ok())
        .unwrap_or("")
        .to_ascii_lowercase();

    let text = if ct.starts_with("application/json") {
        let payload: SummarizeRequest = serde_json::from_slice(&body)
            .map_err(|e| (axum::http::StatusCode::BAD_REQUEST, format!("invalid json: {e}")))?;
        payload.text
    } else {
        String::from_utf8_lossy(&body).to_string()
    };

    if text.trim().is_empty() {
        return Err((axum::http::StatusCode::BAD_REQUEST, "text is empty".into()));
    }

    let summary = groq::summarize(&state.http, &state.groq_api_key, &state.groq_model, &text)
        .await
        .map_err(internal_error)?;

    let id = state
        .db
        .insert_summary(&text, &summary)
        .await
        .map_err(internal_error)?;

    Ok(Json(SummarizeResponse { id, summary }))
}

async fn list(State(state): State<Arc<AppState>>) -> Result<Json<ListResponse>, (axum::http::StatusCode, String)> {
    let rows: Vec<SummaryListItem> = state.db.list_summaries(100).await.map_err(internal_error)?;
    let items = rows
        .into_iter()
        .map(|r| ListItem {
            id: r.id,
            created_at: r.created_at,
            summary_preview: r.summary.chars().take(80).collect(),
        })
        .collect();
    Ok(Json(ListResponse { items }))
}

async fn detail(State(state): State<Arc<AppState>>, Path(id): Path<i64>) -> Result<Json<DetailResponse>, (axum::http::StatusCode, String)> {
    let item = state
        .db
        .get_summary(id)
        .await
        .map_err(internal_error)?
        .ok_or((axum::http::StatusCode::NOT_FOUND, "not found".into()))?;
    Ok(Json(DetailResponse { item }))
}

fn internal_error<E: std::fmt::Display>(err: E) -> (axum::http::StatusCode, String) {
    (axum::http::StatusCode::INTERNAL_SERVER_ERROR, err.to_string())
}

#[derive(Serialize)]
struct DebugInfo {
    database_url: String,
    db_file_path: Option<String>,
    file_exists: bool,
    file_size: Option<u64>,
    total_rows: i64,
}

async fn debug(State(state): State<Arc<AppState>>) -> Result<Json<DebugInfo>, (axum::http::StatusCode, String)> {
    let total_rows = state.db.count().await.map_err(internal_error)?;
    let path = db::db_file_path_from_url(&state.database_url);
    let (file_exists, file_size) = if let Some(p) = path.as_deref() {
        if let Ok(md) = std::fs::metadata(p) {
            (true, Some(md.len()))
        } else {
            (false, None)
        }
    } else {
        (false, None)
    };
    Ok(Json(DebugInfo {
        database_url: state.database_url.clone(),
        db_file_path: path.map(|p| p.to_string_lossy().into_owned()),
        file_exists,
        file_size,
        total_rows,
    }))
}
