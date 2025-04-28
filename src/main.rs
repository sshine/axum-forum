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

    let forum_db_path = ok_or_exit(env_var_default("FORUM_DB_PATH", "forum.db".to_string()));
    let database = Arc::new(Mutex::new(ok_or_exit(db_connection(&forum_db_path))));
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

    let forum_host = ok_or_exit(env_var_default("FORUM_HOST", "127.0.0.1".to_string()));
    let forum_port = ok_or_exit(env_var_parse_default("FORUM_PORT", 3000));
    tracing::info!("Listening on http://{}:{}", forum_host, forum_port);
    let listener = match tokio::net::TcpListener::bind((forum_host, forum_port)).await {
        Ok(listener) => listener,
        Err(why) => {
            tracing::error!("{}", why);
            std::process::exit(1);
        }
    };

    ok_or_exit(axum::serve(listener, app).await)
}

fn db_connection(forum_db_path: &str) -> ForumResult<rusqlite::Connection> {
    let conn = rusqlite::Connection::open(forum_db_path).map_err(ForumError::DatabaseError)?;
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

fn env_var_default<K>(key: K, default: String) -> ForumResult<String>
where
    K: AsRef<std::ffi::OsStr>,
{
    match std::env::var(key) {
        Ok(value) => Ok(value),
        Err(std::env::VarError::NotPresent) => Ok(default),
        Err(otherwise) => Err(ForumError::EnvVarError(otherwise)),
    }
}

fn env_var_parse_default<K, T>(key: K, default: T) -> ForumResult<T>
where
    K: AsRef<std::ffi::OsStr>,
    T: std::str::FromStr,
    <T as std::str::FromStr>::Err: std::fmt::Debug,
{
    let value = match std::env::var(key) {
        Ok(value) => Ok(value),
        Err(std::env::VarError::NotPresent) => return Ok(default),
        Err(otherwise) => Err(ForumError::EnvVarError(otherwise)),
    }?;

    let thing = value
        .parse::<T>()
        .map_err(|why| ForumError::EnvParseError(format!("{:?}", why)))?;
    Ok(thing)
}

fn ok_or_exit<T, E: std::fmt::Display>(result: Result<T, E>) -> T {
    match result {
        Ok(v) => v,
        Err(why) => {
            tracing::error!("{}", why);
            std::process::exit(1);
        }
    }
}
