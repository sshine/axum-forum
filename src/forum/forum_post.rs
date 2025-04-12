use std::collections::HashMap;

use chrono::NaiveDateTime;
use serde::{Deserialize, Serialize};

use crate::{ForumError, ForumResult};

pub static FORUM_POSTS_SQL: &'static str = "
CREATE TABLE IF NOT EXISTS forum_posts (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    root_id INTEGER,
    parent_id INTEGER,
    created_at DATETIME DEFAULT CURRENT_TIMESTAMP,
    deleted_at DATETIME DEFAULT NULL,
    author TEXT NOT NULL,
    message TEXT NOT NULL,
    FOREIGN KEY (root_id) REFERENCES forum_posts(id) ON DELETE CASCADE,
    FOREIGN KEY (parent_id) REFERENCES forum_posts(id) ON DELETE CASCADE
);
";

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ForumPost {
    pub id: usize,
    pub root_id: Option<usize>,
    pub parent_id: Option<usize>,
    pub author: String,
    pub created_at: NaiveDateTime,
    pub deleted_at: Option<NaiveDateTime>,
    pub message: String,
}

impl ForumPost {
    pub fn get_from_db(row: &rusqlite::Row<'_>) -> Result<ForumPost, rusqlite::Error> {
        let mut found_post = ForumPost {
            id: row.get(0)?,
            root_id: row.get(1)?,
            parent_id: row.get(2)?,
            created_at: row.get(3)?,
            deleted_at: row.get(4)?,
            author: row.get(5)?,
            message: row.get(6)?,
        };

        if found_post.deleted_at != None {
            found_post.message = "This message was deleted.".to_string();
        }

        Ok(found_post)
    }

    pub fn get(conn: &rusqlite::Connection, id: usize) -> ForumResult<Self> {
        let mut stmt = conn
            .prepare(
                "
                SELECT id, root_id, parent_id, created_at, deleted_at, author, message
                FROM forum_posts
                WHERE id = ?1
            ",
            )
            .map_err(ForumError::DatabaseError)?;

        let found_post = stmt
            .query_row([id], |row| ForumPost::get_from_db(row))
            .map_err(ForumError::DatabaseError)?;

        Ok(found_post)
    }

    pub fn get_ops(conn: &rusqlite::Connection) -> ForumResult<Vec<Self>> {
        let mut stmt = conn
            .prepare(
                "
                SELECT id, root_id, parent_id, created_at, deleted_at, author, message
                FROM forum_posts
                WHERE root_id IS NULL
                ORDER BY created_at DESC
                ",
            )
            .map_err(ForumError::DatabaseError)?;

        let posts = stmt
            .query_map([], |row| ForumPost::get_from_db(row))
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
            deleted_at: None,
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
            deleted_at: None,
            message,
        };

        Ok(forum_reply)
    }

    pub fn soft_delete_post(conn: &rusqlite::Connection, id: usize) -> ForumResult<()> {
        let deleted_at = chrono::Local::now().naive_local();
        let affected_rows = conn
            .execute(
                "UPDATE forum_posts SET deleted_at = ?1 WHERE id = ?2 AND deleted_at IS NULL",
                (deleted_at, id),
            )
            .map_err(ForumError::DatabaseError)?;

        if affected_rows == 0 {
            return Err(ForumError::NotFound(id));
        }

        Ok(())
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PostTreeNode {
    pub post: ForumPost,
    pub replies: Vec<PostTreeNode>,
}

impl PostTreeNode {
    pub fn build_tree(conn: &rusqlite::Connection, post_id: usize) -> ForumResult<Vec<Self>> {
        let post = ForumPost::get(conn, post_id)?;
        let root_id = post.root_id.unwrap_or(post_id); // due to how OP's are saved, the OP.root_id != OP.id
        let mut stmt = conn
            .prepare(
                "
            SELECT id, root_id, parent_id, created_at, deleted_at, author, message
            FROM forum_posts
            WHERE root_id = ?1
            ORDER BY created_at ASC
            ",
            )
            .map_err(ForumError::DatabaseError)?;

        let posts = stmt
            .query_map([root_id], |row| Ok(ForumPost::get_from_db(row)?))
            .map_err(ForumError::DatabaseError)?
            .collect::<Result<Vec<_>, _>>()
            .map_err(ForumError::DatabaseError)?;

        // builds a hashmap from post ids -> vecs of children
        let mut grouped: HashMap<usize, Vec<ForumPost>> = HashMap::new();
        for post in posts {
            grouped
                .entry(post.parent_id.unwrap())
                .or_default()
                .push(post);
        }

        // recursively builds tree of posts using the hashmap that descend from a given parent_id
        fn helper(
            parent_id: usize,
            grouped: &mut HashMap<usize, Vec<ForumPost>>,
        ) -> Vec<PostTreeNode> {
            grouped
                .remove(&parent_id)
                .unwrap_or_default()
                .into_iter()
                .map(|post| {
                    let replies = helper(post.id, grouped);
                    PostTreeNode { post, replies }
                })
                .collect()
        }

        let tree: Vec<PostTreeNode> = helper(post_id, &mut grouped);
        Ok(tree)
    }
}
