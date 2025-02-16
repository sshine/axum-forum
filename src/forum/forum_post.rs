use chrono::NaiveDateTime;
use serde::{Deserialize, Serialize};

use crate::{ForumError, ForumResult};

pub static FORUM_POSTS_SQL: &'static str = "
CREATE TABLE IF NOT EXISTS forum_posts (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    root_id INTEGER,
    parent_id INTEGER,
    created_at DATETIME DEFAULT CURRENT_TIMESTAMP,
    author TEXT NOT NULL,
    message TEXT NOT NULL,
    FOREIGN KEY (root_id) REFERENCES forum_posts(id) ON DELETE CASCADE,
    FOREIGN KEY (parent_id) REFERENCES forum_posts(id) ON DELETE CASCADE
);
";

#[derive(Serialize, Deserialize)]
pub struct ForumPost {
    pub id: usize,
    pub root_id: Option<usize>,
    pub parent_id: Option<usize>,
    pub author: String,
    pub created_at: NaiveDateTime,
    pub message: String,
}

impl ForumPost {
    pub fn get(conn: &rusqlite::Connection, id: usize) -> ForumResult<Self> {
        let mut stmt = conn
            .prepare(
                "
                SELECT id, root_id, parent_id, created_at, author, message
                FROM forum_posts
                WHERE id = ?1
            ",
            )
            .map_err(ForumError::DatabaseError)?;

        let found_post = stmt
            .query_row([id], |row| {
                Ok(ForumPost {
                    id: row.get(0)?,
                    root_id: row.get(1)?,
                    parent_id: row.get(2)?,
                    created_at: row.get(3)?,
                    author: row.get(4)?,
                    message: row.get(5)?,
                })
            })
            .map_err(ForumError::DatabaseError)?;

        Ok(found_post)
    }

    pub fn get_all(conn: &rusqlite::Connection) -> ForumResult<Vec<Self>> {
        let mut stmt = conn
            .prepare(
                "
                SELECT id, root_id, parent_id, created_at, author, message
                FROM forum_posts
                ORDER BY created_at DESC
                ",
            )
            .map_err(ForumError::DatabaseError)?;

        let posts = stmt
            .query_map([], |row| {
                Ok(ForumPost {
                    id: row.get(0)?,
                    root_id: row.get(1)?,
                    parent_id: row.get(2)?,
                    created_at: row.get(3)?,
                    author: row.get(4)?,
                    message: row.get(5)?,
                })
            })
            .map_err(ForumError::DatabaseError)?
            .collect::<Result<Vec<_>, _>>()
            .map_err(ForumError::DatabaseError)?;

        Ok(posts)
    }

    pub fn post_save(
        conn: &rusqlite::Connection,
        author: String,
        message: String,
    ) -> ForumResult<Self> {
        let created_at = chrono::Local::now().naive_local();

        conn.execute(
            "INSERT INTO forum_posts (created_at, author, message) VALUES (?1, ?2, ?3)",
            (&created_at, &author, &message),
        )
        .map_err(ForumError::DatabaseError)?;

        let id = conn.last_insert_rowid() as usize;

        let forum_post = ForumPost {
            id,
            root_id: None,
            parent_id: None,
            author,
            created_at,
            message,
        };

        Ok(forum_post)
    }

    pub fn reply_save(
        &self,
        conn: &rusqlite::Connection,
        author: String,
        message: String,
    ) -> ForumResult<Self> {
        let root_id = self.root_id.unwrap_or(self.id);
        let parent_id = self.id;
        let created_at = chrono::Local::now().naive_local();

        let id = conn.execute(
            "INSERT INTO forum_posts (root_id, parent_id, created_at, author, message) VALUES (?1, ?2, ?3, ?4, ?5)",
            (root_id, parent_id, created_at, &author, &message),
        )
        .map_err(ForumError::DatabaseError)?;

        let forum_reply = ForumPost {
            id,
            root_id: Some(root_id),
            parent_id: Some(parent_id),
            author,
            created_at,
            message,
        };

        Ok(forum_reply)
    }
}
