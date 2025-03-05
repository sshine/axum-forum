use std::sync::{Arc, Mutex};

use axum::{
    Router,
    routing::{get, post},
};

mod error;
mod forum;

pub use error::{ForumError, ForumResult};

#[derive(Clone)]
struct AppState {
    pub template: minijinja::Environment<'static>,
    pub database: Arc<Mutex<rusqlite::Connection>>,
}

#[tokio::main]
async fn main() {
    let template = template_setup().unwrap();
    let database = Arc::new(Mutex::new(db_connection()));
    let app_state = AppState { template, database };
    let app = Router::new()
        .route("/", get(forum::show_posts))
        .route("/post", get(forum::show_create_post))
        .route("/post", post(forum::handle_create_post))
        .route("/post/{post_id}", get(forum::show_post))
        .route("/reply/{post_id}", get(forum::show_create_reply))
        .route("/reply/{post_id}", post(forum::handle_create_reply))
        .route("/assets/base.css", get(forum::base_css))
        .with_state(app_state);
    let addr = "0.0.0.0:3002";
    println!("Listening on http://{}", addr);
    let listener = tokio::net::TcpListener::bind(addr).await.unwrap();
    axum::serve(listener, app).await.unwrap();
}

fn db_connection() -> rusqlite::Connection {
    let conn = rusqlite::Connection::open("forum.db").unwrap();
    conn.execute_batch(forum::FORUM_POSTS_SQL).unwrap();
    conn
}

pub fn template_setup() -> ForumResult<minijinja::Environment<'static>> {
    let mut env = minijinja::Environment::new();

    env.add_template("base", include_str!("../templates/base.jinja"))
        .map_err(ForumError::TemplateError)?;
    env.add_template("show_posts", include_str!("../templates/show_posts.jinja"))
        .map_err(ForumError::TemplateError)?;
    env.add_template(
        "show_create",
        include_str!("../templates/show_create.jinja"),
    )
    .map_err(ForumError::TemplateError)?;
    env.add_template("show_post", include_str!("../templates/show_post.jinja"))
        .map_err(ForumError::TemplateError)?;
    env.add_template(
        "show_reply_create",
        include_str!("../templates/show_reply.jinja"),
    )
    .map_err(ForumError::TemplateError)?;
    Ok(env)
}
