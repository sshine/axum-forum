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
    let env_filter = tracing_subscriber::EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| format!("{}=debug", env!("CARGO_CRATE_NAME")).into());

    let fmt_layer = tracing_subscriber::fmt::layer()
        .compact()
        .with_ansi(true)
        .with_level(true)
        .with_file(true)
        .with_line_number(true);

    tracing_subscriber::registry()
        .with(env_filter)
        .with(fmt_layer)
        .init();

    let template = match template_setup() {
        Ok(t) => t,
        Err(why) => {
            tracing::error!("{}", why);
            std::process::exit(1);
        }
    };

    let database = match db_connection() {
        Ok(db) => Arc::new(Mutex::new(db)),
        Err(why) => {
            tracing::error!("{}", why);
            std::process::exit(1);
        }
    };

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
    let listener = match tokio::net::TcpListener::bind(addr).await {
        Ok(listener) => listener,
        Err(why) => {
            tracing::error!("{}", why);
            std::process::exit(1);
        }
    };

    if let Err(why) = axum::serve(listener, app).await {
        tracing::error!("{}", why);
        std::process::exit(1);
    }
}

fn db_connection() -> ForumResult<rusqlite::Connection> {
    let conn = rusqlite::Connection::open("forum.db").map_err(ForumError::DatabaseError)?;
    conn.execute_batch(forum::FORUM_POSTS_SQL)
        .map_err(ForumError::DatabaseError)?;
    Ok(conn)
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
