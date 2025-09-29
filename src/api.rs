use crate::db::{self, Db, Summary, SummaryListItem};
use crate::groq;
use anyhow::Result;
use axum::{
    body::Bytes,
    extract::{Path, State},
    http::{header, HeaderMap, StatusCode},
    response::IntoResponse,
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
) -> Result<Json<SummarizeResponse>, ApiError> {
    // Accept either application/json {"text": "..."} or text/plain raw body
    let ct = headers
        .get(header::CONTENT_TYPE)
        .and_then(|v| v.to_str().ok())
        .unwrap_or("")
        .to_ascii_lowercase();

    let text_raw = if ct.starts_with("application/json") {
        let payload: SummarizeRequest = serde_json::from_slice(&body)
            .map_err(|e| ApiError::bad_request("invalid_json", format!("invalid json: {e}")))?;
        payload.text
    } else {
        // default to text/plain
        String::from_utf8_lossy(&body).to_string()
    };

    // Validation rules (lightweight)
    let max_bytes: usize = std::env::var("MAX_INPUT_BYTES").ok().and_then(|v| v.parse().ok()).unwrap_or(32 * 1024);
    let max_chars: usize = std::env::var("MAX_INPUT_CHARS").ok().and_then(|v| v.parse().ok()).unwrap_or(8000);
    let min_chars: usize = std::env::var("MIN_INPUT_CHARS").ok().and_then(|v| v.parse().ok()).unwrap_or(5);

    if body.len() > max_bytes {
        return Err(ApiError::payload_too_large(format!("body too large: {} bytes (max {})", body.len(), max_bytes)));
    }

    let text = text_raw.trim();
    let char_len = text.chars().count();
    if char_len == 0 {
        return Err(ApiError::bad_request("text_empty", "text is empty"));
    }
    if char_len < min_chars {
        return Err(ApiError::unprocessable("text_too_short", format!("text too short: {} chars (min {})", char_len, min_chars)));
    }
    if char_len > max_chars {
        return Err(ApiError::unprocessable("text_too_long", format!("text too long: {} chars (max {})", char_len, max_chars)));
    }

    let summary = groq::summarize(&state.http, &state.groq_api_key, &state.groq_model, text)
        .await
        .map_err(ApiError::internal)?;

    let id = state
        .db
        .insert_summary(text, &summary)
        .await
        .map_err(ApiError::internal)?;

    Ok(Json(SummarizeResponse { id, summary }))
}

async fn list(State(state): State<Arc<AppState>>) -> Result<Json<ListResponse>, ApiError> {
    let rows: Vec<SummaryListItem> = state.db.list_summaries(100).await.map_err(ApiError::internal)?;
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

async fn detail(State(state): State<Arc<AppState>>, Path(id): Path<i64>) -> Result<Json<DetailResponse>, ApiError> {
    if id <= 0 {
        return Err(ApiError::bad_request("invalid_id", "id must be positive"));
    }
    let item = state
        .db
        .get_summary(id)
        .await
        .map_err(ApiError::internal)?
        .ok_or(ApiError::not_found("not_found", "summary not found"))?;
    Ok(Json(DetailResponse { item }))
}

// ---------- Error helpers ----------

#[derive(Serialize)]
struct ErrorBody<'a> {
    code: &'a str,
    message: String,
}

#[derive(Debug)]
pub struct ApiError {
    status: StatusCode,
    code: &'static str,
    message: String,
}

impl ApiError {
    fn new(status: StatusCode, code: &'static str, message: impl Into<String>) -> Self {
        Self { status, code, message: message.into() }
    }
    fn bad_request(code: &'static str, msg: impl Into<String>) -> Self {
        Self::new(StatusCode::BAD_REQUEST, code, msg)
    }
    fn unprocessable(code: &'static str, msg: impl Into<String>) -> Self {
        Self::new(StatusCode::UNPROCESSABLE_ENTITY, code, msg)
    }
    fn payload_too_large(msg: impl Into<String>) -> Self {
        Self::new(StatusCode::PAYLOAD_TOO_LARGE, "payload_too_large", msg)
    }
    fn not_found(code: &'static str, msg: impl Into<String>) -> Self {
        Self::new(StatusCode::NOT_FOUND, code, msg)
    }
    fn internal(e: impl std::fmt::Display) -> Self {
        Self::new(StatusCode::INTERNAL_SERVER_ERROR, "internal", e.to_string())
    }
}

impl IntoResponse for ApiError {
    fn into_response(self) -> axum::response::Response {
        let body = Json(ErrorBody { code: self.code, message: self.message });
        (self.status, body).into_response()
    }
}

#[derive(Serialize)]
struct DebugInfo {
    database_url: String,
    db_file_path: Option<String>,
    file_exists: bool,
    file_size: Option<u64>,
    total_rows: i64,
}

async fn debug(State(state): State<Arc<AppState>>) -> Result<Json<DebugInfo>, ApiError> {
    let total_rows = state.db.count().await.map_err(ApiError::internal)?;
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
