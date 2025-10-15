use crate::db::{
    self, Account, Db, Group, GroupUser, GroupWithRole, SharedNote,
};
use axum::{
    extract::{Json as JsonPayload, Path, State},
    http::StatusCode,
    response::IntoResponse,
    routing::{get, patch, post},
    Json, Router,
};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::sync::Arc;

#[derive(Clone)]
pub struct AppState {
    pub db: Db,
    pub database_url: String,
}

pub fn routes(state: AppState) -> Router {
    Router::new()
        // accounts
        .route("/api/accounts", get(list_accounts).post(create_account))
        .route("/api/accounts/:id/groups", get(list_groups_for_user))
        // groups
        .route("/api/groups", post(create_group))
        .route("/api/groups/:id", get(get_group))
        .route("/api/groups/:id/users", get(list_group_members).post(add_user_to_group))
        .route(
            "/api/groups/:id/notes",
            get(list_group_notes).post(create_group_note).delete(clear_group_notes),
        )
        // notes
        .route("/api/notes/:id", patch(update_note_content).delete(delete_note))
        .route("/api/notes/:id/position", patch(update_note_position))
        // misc
        .route("/api/debug", get(debug))
        .with_state(Arc::new(state))
}

// -------------------------------------------------------------------
// Accounts

async fn list_accounts(State(state): State<Arc<AppState>>) -> Result<Json<AccountsResponse>, ApiError> {
    let accounts = state
        .db
        .list_accounts()
        .await
        .map_err(ApiError::internal)?
        .into_iter()
        .map(AccountSummary::from)
        .collect();
    Ok(Json(AccountsResponse { accounts }))
}

async fn create_account(
    State(state): State<Arc<AppState>>,
    JsonPayload(payload): JsonPayload<CreateAccountRequest>,
) -> Result<Json<AccountSummary>, ApiError> {
    let name = payload.name.trim();
    let email = payload.email.trim();
    let password = payload.password.trim();

    if name.is_empty() {
        return Err(ApiError::bad_request("name_empty", "名前を入力してください"));
    }
    if email.is_empty() {
        return Err(ApiError::bad_request("email_empty", "メールアドレスを入力してください"));
    }
    if !email.contains('@') {
        return Err(ApiError::unprocessable("email_invalid", "メールアドレスの形式が正しくありません"));
    }
    if password.len() < 6 {
        return Err(ApiError::unprocessable("password_short", "パスワードは6文字以上にしてください"));
    }

    let hash = hash_password(password);
    let id = state
        .db
        .create_account(name, email, &hash)
        .await
        .map_err(|e| ApiError::internal(format!("アカウント作成に失敗しました: {e}")))?;

    let account = state
        .db
        .get_account(id)
        .await
        .map_err(ApiError::internal)?
        .ok_or_else(|| ApiError::internal("作成したアカウントが見つかりません"))?;

    Ok(Json(AccountSummary::from(account)))
}

// -------------------------------------------------------------------
// Groups

async fn create_group(
    State(state): State<Arc<AppState>>,
    JsonPayload(payload): JsonPayload<CreateGroupRequest>,
) -> Result<Json<GroupSummary>, ApiError> {
    if payload.group_name.trim().is_empty() {
        return Err(ApiError::bad_request("group_name_empty", "グループ名を入力してください"));
    }
    if payload.created_by <= 0 {
        return Err(ApiError::bad_request("created_by_invalid", "作成ユーザーIDが不正です"));
    }

    ensure_account_exists(&state.db, payload.created_by).await?;

    let id = state
        .db
        .create_group(payload.group_name.trim(), payload.created_by)
        .await
        .map_err(ApiError::internal)?;

    let group = state
        .db
        .get_group(id)
        .await
        .map_err(ApiError::internal)?
        .ok_or_else(|| ApiError::internal("作成したグループが見つかりません"))?;

    Ok(Json(GroupSummary::from(group)))
}

async fn get_group(
    State(state): State<Arc<AppState>>,
    Path(id): Path<i64>,
) -> Result<Json<GroupSummary>, ApiError> {
    if id <= 0 {
        return Err(ApiError::bad_request("invalid_id", "グループIDが不正です"));
    }
    let group = state
        .db
        .get_group(id)
        .await
        .map_err(ApiError::internal)?
        .ok_or_else(|| ApiError::not_found("group_not_found", "グループが見つかりません"))?;
    Ok(Json(GroupSummary::from(group)))
}

async fn list_groups_for_user(
    State(state): State<Arc<AppState>>,
    Path(user_id): Path<i64>,
) -> Result<Json<GroupsResponse>, ApiError> {
    if user_id <= 0 {
        return Err(ApiError::bad_request("invalid_user_id", "ユーザーIDが不正です"));
    }
    ensure_account_exists(&state.db, user_id).await?;
    let groups = state
        .db
        .list_groups_for_user(user_id)
        .await
        .map_err(ApiError::internal)?
        .into_iter()
        .map(GroupMembership::from)
        .collect();
    Ok(Json(GroupsResponse { groups }))
}

async fn add_user_to_group(
    State(state): State<Arc<AppState>>,
    Path(group_id): Path<i64>,
    JsonPayload(payload): JsonPayload<JoinGroupRequest>,
) -> Result<StatusCode, ApiError> {
    if group_id <= 0 {
        return Err(ApiError::bad_request("invalid_group_id", "グループIDが不正です"));
    }
    if payload.user_id <= 0 {
        return Err(ApiError::bad_request("invalid_user_id", "ユーザーIDが不正です"));
    }
    ensure_account_exists(&state.db, payload.user_id).await?;
    ensure_group_exists(&state.db, group_id).await?;

    let role = payload.role.unwrap_or_else(|| "member".to_string());
    if !matches!(role.as_str(), "owner" | "member") {
        return Err(ApiError::unprocessable("invalid_role", "role は owner か member にしてください"));
    }

    state
        .db
        .add_user_to_group(group_id, payload.user_id, &role)
        .await
        .map_err(ApiError::internal)?;

    Ok(StatusCode::NO_CONTENT)
}

async fn list_group_members(
    State(state): State<Arc<AppState>>,
    Path(group_id): Path<i64>,
) -> Result<Json<GroupMembersResponse>, ApiError> {
    if group_id <= 0 {
        return Err(ApiError::bad_request("invalid_group_id", "グループIDが不正です"));
    }
    ensure_group_exists(&state.db, group_id).await?;
    let members = state
        .db
        .list_group_members(group_id)
        .await
        .map_err(ApiError::internal)?;
    Ok(Json(GroupMembersResponse { members }))
}

// -------------------------------------------------------------------
// Notes

async fn list_group_notes(
    State(state): State<Arc<AppState>>,
    Path(group_id): Path<i64>,
) -> Result<Json<NotesResponse>, ApiError> {
    if group_id <= 0 {
        return Err(ApiError::bad_request("invalid_group_id", "グループIDが不正です"));
    }
    ensure_group_exists(&state.db, group_id).await?;
    let notes = state
        .db
        .list_notes_for_group(group_id)
        .await
        .map_err(ApiError::internal)?;
    Ok(Json(NotesResponse { notes }))
}

async fn create_group_note(
    State(state): State<Arc<AppState>>,
    Path(group_id): Path<i64>,
    JsonPayload(payload): JsonPayload<CreateNoteRequest>,
) -> Result<Json<CreateNoteResponse>, ApiError> {
    if group_id <= 0 {
        return Err(ApiError::bad_request("invalid_group_id", "グループIDが不正です"));
    }
    ensure_group_exists(&state.db, group_id).await?;

    if let Some(author_id) = payload.created_by {
        ensure_account_exists(&state.db, author_id).await?;
        let belongs = state
            .db
            .is_user_in_group(group_id, author_id)
            .await
            .map_err(ApiError::internal)?;
        if !belongs {
            return Err(ApiError::unprocessable(
                "not_member",
                "このユーザーはグループに参加していません",
            ));
        }
    }

    let color = normalize_color(payload.color.as_deref());
    let width = payload.width.unwrap_or(200.0);
    let height = payload.height.unwrap_or(150.0);
    let z_index = payload.z_index.unwrap_or(0);

    let note_id = state
        .db
        .create_note(
            payload.title.as_deref(),
            payload.content.as_deref(),
            &color,
            payload.x,
            payload.y,
            width,
            height,
            z_index,
            payload.created_by,
            group_id,
            payload.can_edit.unwrap_or(false),
        )
        .await
        .map_err(ApiError::internal)?;

    Ok(Json(CreateNoteResponse { id: note_id }))
}

async fn update_note_position(
    State(state): State<Arc<AppState>>,
    Path(note_id): Path<i64>,
    JsonPayload(payload): JsonPayload<UpdateNotePositionRequest>,
) -> Result<StatusCode, ApiError> {
    if note_id <= 0 {
        return Err(ApiError::bad_request("invalid_note_id", "付箋IDが不正です"));
    }
    let updated = state
        .db
        .update_note_position(
            note_id,
            payload.x,
            payload.y,
            payload.width.unwrap_or(200.0),
            payload.height.unwrap_or(150.0),
            payload.z_index.unwrap_or(0),
        )
        .await
        .map_err(ApiError::internal)?;
    if updated {
        Ok(StatusCode::NO_CONTENT)
    } else {
        Err(ApiError::not_found("note_not_found", "付箋が見つかりません"))
    }
}

async fn update_note_content(
    State(state): State<Arc<AppState>>,
    Path(note_id): Path<i64>,
    JsonPayload(payload): JsonPayload<UpdateNoteContentRequest>,
) -> Result<StatusCode, ApiError> {
    if note_id <= 0 {
        return Err(ApiError::bad_request("invalid_note_id", "付箋IDが不正です"));
    }

    let color = normalize_color(payload.color.as_deref());

    let updated = state
        .db
        .update_note_content(
            note_id,
            payload.title.as_deref(),
            payload.content.as_deref(),
            &color,
        )
        .await
        .map_err(ApiError::internal)?;

    if updated {
        Ok(StatusCode::NO_CONTENT)
    } else {
        Err(ApiError::not_found("note_not_found", "付箋が見つかりません"))
    }
}

async fn delete_note(
    State(state): State<Arc<AppState>>,
    Path(note_id): Path<i64>,
) -> Result<StatusCode, ApiError> {
    if note_id <= 0 {
        return Err(ApiError::bad_request("invalid_note_id", "付箋IDが不正です"));
    }
    let deleted = state
        .db
        .delete_note(note_id)
        .await
        .map_err(ApiError::internal)?;
    if deleted {
        Ok(StatusCode::NO_CONTENT)
    } else {
        Err(ApiError::not_found("note_not_found", "付箋が見つかりません"))
    }
}

async fn clear_group_notes(
    State(state): State<Arc<AppState>>,
    Path(group_id): Path<i64>,
) -> Result<Json<ClearResponse>, ApiError> {
    if group_id <= 0 {
        return Err(ApiError::bad_request("invalid_group_id", "グループIDが不正です"));
    }
    ensure_group_exists(&state.db, group_id).await?;
    let removed = state
        .db
        .clear_notes_for_group(group_id)
        .await
        .map_err(ApiError::internal)?;
    Ok(Json(ClearResponse { removed }))
}

// -------------------------------------------------------------------
// Debug

#[derive(Serialize)]
struct DebugInfo {
    database_url: String,
    db_file_path: Option<String>,
    file_exists: bool,
    file_size: Option<u64>,
    total_notes: i64,
}

async fn debug(State(state): State<Arc<AppState>>) -> Result<Json<DebugInfo>, ApiError> {
    let total_notes = state.db.count_notes().await.map_err(ApiError::internal)?;
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
        total_notes,
    }))
}

// -------------------------------------------------------------------
// Shared helpers & DTOs

#[derive(Serialize)]
struct AccountSummary {
    id: i64,
    name: String,
    email: String,
    created_at: String,
}

impl From<Account> for AccountSummary {
    fn from(a: Account) -> Self {
        Self {
            id: a.id,
            name: a.name,
            email: a.email,
            created_at: a.created_at,
        }
    }
}

#[derive(Serialize)]
struct AccountsResponse {
    accounts: Vec<AccountSummary>,
}

#[derive(Serialize)]
struct GroupSummary {
    id: i64,
    group_name: String,
    created_by: i64,
    created_at: String,
}

impl From<Group> for GroupSummary {
    fn from(g: Group) -> Self {
        Self {
            id: g.id,
            group_name: g.group_name,
            created_by: g.created_by,
            created_at: g.created_at,
        }
    }
}

#[derive(Serialize)]
struct GroupMembership {
    id: i64,
    group_name: String,
    created_by: i64,
    created_at: String,
    role: String,
}

impl From<GroupWithRole> for GroupMembership {
    fn from(g: GroupWithRole) -> Self {
        Self {
            id: g.id,
            group_name: g.group_name,
            created_by: g.created_by,
            created_at: g.created_at,
            role: g.role,
        }
    }
}

#[derive(Serialize)]
struct GroupsResponse {
    groups: Vec<GroupMembership>,
}

#[derive(Serialize)]
struct GroupMembersResponse {
    members: Vec<GroupUser>,
}

#[derive(Serialize)]
struct NotesResponse {
    notes: Vec<SharedNote>,
}

#[derive(Serialize)]
struct CreateNoteResponse {
    id: i64,
}

#[derive(Serialize)]
struct ClearResponse {
    removed: u64,
}

#[derive(Deserialize)]
struct CreateAccountRequest {
    name: String,
    email: String,
    password: String,
}

#[derive(Deserialize)]
struct CreateGroupRequest {
    group_name: String,
    created_by: i64,
}

#[derive(Deserialize)]
struct JoinGroupRequest {
    user_id: i64,
    role: Option<String>,
}

#[derive(Deserialize)]
struct CreateNoteRequest {
    title: Option<String>,
    content: Option<String>,
    color: Option<String>,
    x: f64,
    y: f64,
    width: Option<f64>,
    height: Option<f64>,
    z_index: Option<i64>,
    created_by: Option<i64>,
    can_edit: Option<bool>,
}

#[derive(Deserialize)]
struct UpdateNotePositionRequest {
    x: f64,
    y: f64,
    width: Option<f64>,
    height: Option<f64>,
    z_index: Option<i64>,
}

#[derive(Deserialize)]
struct UpdateNoteContentRequest {
    title: Option<String>,
    content: Option<String>,
    color: Option<String>,
}

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

fn hash_password(password: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(password.as_bytes());
    let digest = hasher.finalize();
    format!("{:x}", digest)
}

fn ensure_account_exists(db: &Db, account_id: i64) -> impl std::future::Future<Output = Result<(), ApiError>> + '_ {
    async move {
        let exists = db
            .get_account(account_id)
            .await
            .map_err(ApiError::internal)?
            .is_some();
        if exists {
            Ok(())
        } else {
            Err(ApiError::not_found("account_not_found", "ユーザーが存在しません"))
        }
    }
}

fn ensure_group_exists(db: &Db, group_id: i64) -> impl std::future::Future<Output = Result<(), ApiError>> + '_ {
    async move {
        let exists = db
            .get_group(group_id)
            .await
            .map_err(ApiError::internal)?
            .is_some();
        if exists {
            Ok(())
        } else {
            Err(ApiError::not_found("group_not_found", "グループが存在しません"))
        }
    }
}

fn normalize_color(input: Option<&str>) -> String {
    let default = "#FFFF88".to_string();
    let Some(raw) = input else { return default; };
    let trimmed = raw.trim();
    if trimmed.is_empty() {
        return default;
    }
    if trimmed.starts_with('#') && trimmed.len() == 7 && trimmed.chars().skip(1).all(|c| c.is_ascii_hexdigit()) {
        return trimmed.to_uppercase();
    }
    match trimmed.to_lowercase().as_str() {
        "yellow" => "#FFFF88".to_string(),
        "pink" => "#FBCFE8".to_string(),
        "green" => "#BBF7D0".to_string(),
        "blue" => "#BFDBFE".to_string(),
        "orange" => "#FED7AA".to_string(),
        "purple" => "#E9D5FF".to_string(),
        _ => default,
    }
}
