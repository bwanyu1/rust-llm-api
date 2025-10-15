use anyhow::Result;
use serde::Serialize;
use sqlx::{sqlite::SqlitePoolOptions, FromRow, Pool, Sqlite, Transaction};

#[derive(Clone)]
pub struct Db {
    pub pool: Pool<Sqlite>,
}

#[derive(FromRow, Debug, Clone, Serialize)]
pub struct Account {
    pub id: i64,
    pub name: String,
    pub email: String,
    #[allow(dead_code)]
    #[serde(skip_serializing)]
    pub password_hash: String,
    pub created_at: String,
}

#[derive(FromRow, Debug, Clone, Serialize)]
pub struct Group {
    pub id: i64,
    pub group_name: String,
    pub created_by: i64,
    pub created_at: String,
}

#[derive(FromRow, Debug, Clone, Serialize)]
pub struct GroupWithRole {
    pub id: i64,
    pub group_name: String,
    pub created_by: i64,
    pub created_at: String,
    pub role: String,
}

#[derive(FromRow, Debug, Clone, Serialize)]
pub struct GroupUser {
    pub id: i64,
    pub group_id: i64,
    pub user_id: i64,
    pub role: String,
    pub joined_at: String,
}

#[derive(FromRow, Debug, Clone, Serialize)]
pub struct NoteRecord {
    pub id: i64,
    pub title: Option<String>,
    pub content: Option<String>,
    pub color: String,
    pub x: f64,
    pub y: f64,
    pub width: f64,
    pub height: f64,
    pub z_index: i64,
    pub created_by: Option<i64>,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(FromRow, Debug, Clone, Serialize)]
pub struct SharedNote {
    pub id: i64,
    pub title: Option<String>,
    pub content: Option<String>,
    pub color: String,
    pub x: f64,
    pub y: f64,
    pub width: f64,
    pub height: f64,
    pub z_index: i64,
    pub created_by: Option<i64>,
    pub created_at: String,
    pub updated_at: String,
    pub group_id: i64,
    pub can_edit: bool,
    pub shared_at: String,
}

impl Db {
    pub async fn init(database_url: &str) -> Result<Self> {
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

        sqlx::query("PRAGMA foreign_keys = ON;").execute(&pool).await?;

        // Reset schema to match the specification
        sqlx::query("DROP TABLE IF EXISTS note_shares;").execute(&pool).await?;
        sqlx::query("DROP TABLE IF EXISTS notes;").execute(&pool).await?;
        sqlx::query("DROP TABLE IF EXISTS group_users;").execute(&pool).await?;
        sqlx::query("DROP TABLE IF EXISTS groups;").execute(&pool).await?;
        sqlx::query("DROP TABLE IF EXISTS accounts;").execute(&pool).await?;

        sqlx::query(
            r#"
            CREATE TABLE accounts (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                name TEXT NOT NULL,
                email TEXT NOT NULL UNIQUE,
                password_hash TEXT NOT NULL,
                created_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP
            );
            "#,
        )
        .execute(&pool)
        .await?;

        sqlx::query(
            r#"
            CREATE TABLE groups (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                group_name TEXT NOT NULL,
                created_by INTEGER NOT NULL,
                created_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
                FOREIGN KEY (created_by) REFERENCES accounts(id) ON DELETE RESTRICT
            );
            "#,
        )
        .execute(&pool)
        .await?;

        sqlx::query(
            r#"
            CREATE TABLE group_users (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                group_id INTEGER NOT NULL,
                user_id INTEGER NOT NULL,
                role TEXT NOT NULL DEFAULT 'member',
                joined_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
                FOREIGN KEY (group_id) REFERENCES groups(id) ON DELETE CASCADE,
                FOREIGN KEY (user_id) REFERENCES accounts(id) ON DELETE CASCADE,
                UNIQUE(group_id, user_id)
            );
            "#,
        )
        .execute(&pool)
        .await?;

        sqlx::query(
            r#"
            CREATE TABLE notes (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                title TEXT,
                content TEXT,
                color TEXT NOT NULL DEFAULT '#FFFF88',
                x REAL NOT NULL,
                y REAL NOT NULL,
                width REAL NOT NULL DEFAULT 200,
                height REAL NOT NULL DEFAULT 150,
                z_index INTEGER NOT NULL DEFAULT 0,
                created_by INTEGER,
                created_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
                updated_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
                FOREIGN KEY (created_by) REFERENCES accounts(id) ON DELETE SET NULL
            );
            "#,
        )
        .execute(&pool)
        .await?;

        sqlx::query(
            r#"
            CREATE TABLE note_shares (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                note_id INTEGER NOT NULL,
                group_id INTEGER NOT NULL,
                can_edit INTEGER NOT NULL DEFAULT 0,
                shared_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
                FOREIGN KEY (note_id) REFERENCES notes(id) ON DELETE CASCADE,
                FOREIGN KEY (group_id) REFERENCES groups(id) ON DELETE CASCADE,
                UNIQUE(note_id, group_id)
            );
            "#,
        )
        .execute(&pool)
        .await?;

        Ok(Self { pool })
    }

    // Accounts --------------------------------------------------------

    pub async fn create_account(&self, name: &str, email: &str, password_hash: &str) -> Result<i64> {
        let res = sqlx::query(
            r#"
            INSERT INTO accounts (name, email, password_hash)
            VALUES (?, ?, ?)
            "#,
        )
        .bind(name)
        .bind(email)
        .bind(password_hash)
        .execute(&self.pool)
        .await?;
        Ok(res.last_insert_rowid())
    }

    pub async fn list_accounts(&self) -> Result<Vec<Account>> {
        let rows = sqlx::query_as::<_, Account>(
            r#"
            SELECT id, name, email, password_hash, created_at
            FROM accounts
            ORDER BY created_at ASC
            "#,
        )
        .fetch_all(&self.pool)
        .await?;
        Ok(rows)
    }

    pub async fn get_account(&self, account_id: i64) -> Result<Option<Account>> {
        let row = sqlx::query_as::<_, Account>(
            r#"
            SELECT id, name, email, password_hash, created_at
            FROM accounts
            WHERE id = ?
            "#,
        )
        .bind(account_id)
        .fetch_optional(&self.pool)
        .await?;
        Ok(row)
    }

    // Groups ----------------------------------------------------------

    pub async fn create_group(&self, group_name: &str, created_by: i64) -> Result<i64> {
        let mut tx = self.pool.begin().await?;
        let group_res = sqlx::query(
            r#"
            INSERT INTO groups (group_name, created_by)
            VALUES (?, ?)
            "#,
        )
        .bind(group_name)
        .bind(created_by)
        .execute(&mut *tx)
        .await?;
        let group_id = group_res.last_insert_rowid();

        sqlx::query(
            r#"
            INSERT INTO group_users (group_id, user_id, role)
            VALUES (?, ?, 'owner')
            "#,
        )
        .bind(group_id)
        .bind(created_by)
        .execute(&mut *tx)
        .await?;

        tx.commit().await?;
        Ok(group_id)
    }

    pub async fn list_groups_for_user(&self, user_id: i64) -> Result<Vec<GroupWithRole>> {
        let rows = sqlx::query_as::<_, GroupWithRole>(
            r#"
            SELECT g.id, g.group_name, g.created_by, g.created_at, gu.role
            FROM groups g
            INNER JOIN group_users gu ON gu.group_id = g.id
            WHERE gu.user_id = ?
            ORDER BY g.created_at ASC
            "#,
        )
        .bind(user_id)
        .fetch_all(&self.pool)
        .await?;
        Ok(rows)
    }

    pub async fn add_user_to_group(&self, group_id: i64, user_id: i64, role: &str) -> Result<i64> {
        let res = sqlx::query(
            r#"
            INSERT OR IGNORE INTO group_users (group_id, user_id, role)
            VALUES (?, ?, ?)
            "#,
        )
        .bind(group_id)
        .bind(user_id)
        .bind(role)
        .execute(&self.pool)
        .await?;
        Ok(res.last_insert_rowid())
    }

    pub async fn list_group_members(&self, group_id: i64) -> Result<Vec<GroupUser>> {
        let rows = sqlx::query_as::<_, GroupUser>(
            r#"
            SELECT id, group_id, user_id, role, joined_at
            FROM group_users
            WHERE group_id = ?
            ORDER BY joined_at ASC
            "#,
        )
        .bind(group_id)
        .fetch_all(&self.pool)
        .await?;
        Ok(rows)
    }

    pub async fn get_group(&self, group_id: i64) -> Result<Option<Group>> {
        let row = sqlx::query_as::<_, Group>(
            r#"
            SELECT id, group_name, created_by, created_at
            FROM groups
            WHERE id = ?
            "#,
        )
        .bind(group_id)
        .fetch_optional(&self.pool)
        .await?;
        Ok(row)
    }

    pub async fn is_user_in_group(&self, group_id: i64, user_id: i64) -> Result<bool> {
        let exists: Option<i64> = sqlx::query_scalar(
            r#"
            SELECT 1
            FROM group_users
            WHERE group_id = ? AND user_id = ?
            "#,
        )
        .bind(group_id)
        .bind(user_id)
        .fetch_optional(&self.pool)
        .await?;
        Ok(exists.is_some())
    }

    // Notes -----------------------------------------------------------

    pub async fn create_note(
        &self,
        title: Option<&str>,
        content: Option<&str>,
        color: &str,
        x: f64,
        y: f64,
        width: f64,
        height: f64,
        z_index: i64,
        created_by: Option<i64>,
        group_id: i64,
        can_edit: bool,
    ) -> Result<i64> {
        let mut tx: Transaction<'_, Sqlite> = self.pool.begin().await?;

        let note_res = sqlx::query(
            r#"
            INSERT INTO notes (title, content, color, x, y, width, height, z_index, created_by)
            VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?)
            "#,
        )
        .bind(title)
        .bind(content)
        .bind(color)
        .bind(x)
        .bind(y)
        .bind(width)
        .bind(height)
        .bind(z_index)
        .bind(created_by)
        .execute(&mut *tx)
        .await?;

        let note_id = note_res.last_insert_rowid();

        sqlx::query(
            r#"
            INSERT INTO note_shares (note_id, group_id, can_edit)
            VALUES (?, ?, ?)
            "#,
        )
        .bind(note_id)
        .bind(group_id)
        .bind(if can_edit { 1 } else { 0 })
        .execute(&mut *tx)
        .await?;

        tx.commit().await?;
        Ok(note_id)
    }

    pub async fn list_notes_for_group(&self, group_id: i64) -> Result<Vec<SharedNote>> {
        let rows = sqlx::query_as::<_, SharedNote>(
            r#"
            SELECT
                n.id,
                n.title,
                n.content,
                n.color,
                n.x,
                n.y,
                n.width,
                n.height,
                n.z_index,
                n.created_by,
                n.created_at,
                n.updated_at,
                ns.group_id,
                ns.can_edit as can_edit,
                ns.shared_at
            FROM notes n
            INNER JOIN note_shares ns ON ns.note_id = n.id
            WHERE ns.group_id = ?
            ORDER BY n.z_index ASC, n.updated_at ASC
            "#,
        )
        .bind(group_id)
        .fetch_all(&self.pool)
        .await?;
        Ok(rows)
    }

    pub async fn update_note_position(
        &self,
        note_id: i64,
        x: f64,
        y: f64,
        width: f64,
        height: f64,
        z_index: i64,
    ) -> Result<bool> {
        let res = sqlx::query(
            r#"
            UPDATE notes
            SET x = ?, y = ?, width = ?, height = ?, z_index = ?, updated_at = CURRENT_TIMESTAMP
            WHERE id = ?
            "#,
        )
        .bind(x)
        .bind(y)
        .bind(width)
        .bind(height)
        .bind(z_index)
        .bind(note_id)
        .execute(&self.pool)
        .await?;
        Ok(res.rows_affected() > 0)
    }

    pub async fn update_note_content(
        &self,
        note_id: i64,
        title: Option<&str>,
        content: Option<&str>,
        color: &str,
    ) -> Result<bool> {
        let res = sqlx::query(
            r#"
            UPDATE notes
            SET title = ?, content = ?, color = ?, updated_at = CURRENT_TIMESTAMP
            WHERE id = ?
            "#,
        )
        .bind(title)
        .bind(content)
        .bind(color)
        .bind(note_id)
        .execute(&self.pool)
        .await?;
        Ok(res.rows_affected() > 0)
    }

    pub async fn delete_note(&self, note_id: i64) -> Result<bool> {
        let res = sqlx::query(
            r#"
            DELETE FROM notes WHERE id = ?
            "#,
        )
        .bind(note_id)
        .execute(&self.pool)
        .await?;
        Ok(res.rows_affected() > 0)
    }

    pub async fn clear_notes_for_group(&self, group_id: i64) -> Result<u64> {
        let mut tx = self.pool.begin().await?;

        let note_ids: Vec<i64> = sqlx::query_scalar(
            r#"
            SELECT note_id
            FROM note_shares
            WHERE group_id = ?
            "#,
        )
        .bind(group_id)
        .fetch_all(&mut *tx)
        .await?;

        let mut removed: u64 = 0;
        for note_id in note_ids {
            let res = sqlx::query("DELETE FROM notes WHERE id = ?")
                .bind(note_id)
                .execute(&mut *tx)
                .await?;
            removed += res.rows_affected();
        }

        tx.commit().await?;
        Ok(removed)
    }

    pub async fn count_notes(&self) -> Result<i64> {
        let c: (i64,) = sqlx::query_as("SELECT COUNT(*) as c FROM notes")
            .fetch_one(&self.pool)
            .await?;
        Ok(c.0)
    }

}

pub fn db_file_path_from_url(url: &str) -> Option<std::path::PathBuf> {
    if url.starts_with("sqlite::memory:") {
        return None;
    }
    if let Some(rest) = url.strip_prefix("sqlite://") {
        let p = std::path::PathBuf::from(rest);
        return Some(p);
    }
    None
}
