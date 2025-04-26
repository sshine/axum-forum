mod controller;
mod forum_post;

pub use controller::*;
pub use forum_post::FORUM_POSTS_SQL;

pub type PostId = i64;
