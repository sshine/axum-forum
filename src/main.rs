use std::sync::{Arc, Mutex};

use axum::{
    Router,
    routing::{get, post},
};

mod error;
mod forum;

pub use error::{ForumError, ForumResult};
use tracing_subscriber::{layer::SubscriberExt as _, util::SubscriberInitExt as _};

#[derive(Clone)]
struct AppState {
    pub template: minijinja::Environment<'static>,
    pub database: Arc<Mutex<rusqlite::Connection>>,
}

#[tokio::main]
async fn main() {
    tracing_subscriber::registry()
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| format!("{}=debug", env!("CARGO_CRATE_NAME")).into()),
        )
        .with(tracing_subscriber::fmt::layer())
        .init();

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
        .route("/delete/{post_id}", post(forum::handle_delete_post))
        .route("/assets/base.css", get(forum::base_css))
        .with_state(app_state);

    let addr = "0.0.0.0:3000";
    tracing::info!("Listening on http://{}", addr);
    let listener = tokio::net::TcpListener::bind(addr).await.unwrap();
    axum::serve(listener, app).await.unwrap();
}

fn db_connection() -> rusqlite::Connection {
    let conn = rusqlite::Connection::open("forum.db").unwrap();
    conn.execute_batch(forum::FORUM_POSTS_SQL).unwrap();
    conn
}

macro_rules! add_template {
    ($env:expr, $template_name:expr) => {{
        $env.add_template(
            $template_name,
            include_str!(concat!("../templates/", $template_name, ".jinja")),
        )
        .map_err(ForumError::TemplateError)
    }};
}

pub fn template_setup() -> ForumResult<minijinja::Environment<'static>> {
    let mut env = minijinja::Environment::new();

    add_template!(env, "base")?;
    add_template!(env, "show_posts")?;
    add_template!(env, "show_create")?;
    add_template!(env, "show_post")?;
    add_template!(env, "show_reply")?;

    Ok(env)
}
