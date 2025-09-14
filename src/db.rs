use anyhow::Result;
use serde::Serialize;
use sqlx::{sqlite::SqlitePoolOptions, FromRow, Pool, Sqlite};

#[derive(Clone)]
pub struct Db {
    pub pool: Pool<Sqlite>,
}

#[derive(FromRow, Debug, Clone, Serialize)]
pub struct Summary {
    pub id: i64,
    pub input_text: String,
    pub summary: String,
    pub created_at: String,
}

#[derive(FromRow, Debug, Clone, Serialize)]
pub struct SummaryListItem {
    pub id: i64,
    pub created_at: String,
    pub summary: String,
}

impl Db {
    pub async fn init(database_url: &str) -> Result<Self> {
        // Best-effort: create the SQLite file and its parent directory if using a file-based URL
        if let Some(path) = db_file_path_from_url(database_url) {
            if let Some(parent) = path.parent() {
                if !parent.as_os_str().is_empty() {
                    std::fs::create_dir_all(parent)?;
                }
            }
            if !path.exists() {
                std::fs::File::create(&path)?;
            }
        }

        let pool = SqlitePoolOptions::new()
            .max_connections(5)
            .connect(database_url)
            .await?;

        // Create table if not exists
        sqlx::query(
            r#"
            CREATE TABLE IF NOT EXISTS summaries (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                input_text TEXT NOT NULL,
                summary TEXT NOT NULL,
                created_at TEXT NOT NULL DEFAULT (datetime('now'))
            );
            "#,
        )
        .execute(&pool)
        .await?;

        Ok(Self { pool })
    }

    pub async fn insert_summary(&self, input_text: &str, summary: &str) -> Result<i64> {
        let res = sqlx::query(
            r#"INSERT INTO summaries (input_text, summary) VALUES (?, ?)"#,
        )
        .bind(input_text)
        .bind(summary)
        .execute(&self.pool)
        .await?;
        Ok(res.last_insert_rowid())
    }

    pub async fn get_summary(&self, id: i64) -> Result<Option<Summary>> {
        let rec = sqlx::query_as::<_, Summary>(
            r#"SELECT id, input_text, summary, created_at FROM summaries WHERE id = ?"#,
        )
        .bind(id)
        .fetch_optional(&self.pool)
        .await?;
        Ok(rec)
    }

    pub async fn list_summaries(&self, limit: i64) -> Result<Vec<SummaryListItem>> {
        let rows = sqlx::query_as::<_, SummaryListItem>(
            r#"SELECT id, created_at, summary FROM summaries ORDER BY id DESC LIMIT ?"#,
        )
        .bind(limit)
        .fetch_all(&self.pool)
        .await?;
        Ok(rows)
    }

    pub async fn count(&self) -> Result<i64> {
        let c: (i64,) = sqlx::query_as("SELECT COUNT(*) as c FROM summaries")
            .fetch_one(&self.pool)
            .await?;
        Ok(c.0)
    }
}

pub fn db_file_path_from_url(url: &str) -> Option<std::path::PathBuf> {
    // sqlite::memory: special in-memory DB
    if url.starts_with("sqlite::memory:") { return None; }
    if let Some(rest) = url.strip_prefix("sqlite://") {
        // Absolute path if starts with '/'; otherwise relative to CWD
        let p = std::path::PathBuf::from(rest);
        return Some(p);
    }
    None
}
