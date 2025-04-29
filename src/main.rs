use std::sync::{Arc, Mutex};

use axum::{
    Router,
    routing::{get, post},
};

mod config;
mod error;
mod forum;

use config::AppConfig;
use config_manager::ConfigInit;
pub use error::{ForumError, ForumResult};
use tracing_subscriber::{layer::SubscriberExt as _, util::SubscriberInitExt as _};

#[derive(Clone)]
struct AppState {
    pub config: AppConfig,
    pub template: minijinja::Environment<'static>,
    pub database: Arc<Mutex<rusqlite::Connection>>,
}

#[tokio::main]
async fn main() {
    setup_tracing();
    let config = ok_or_exit(parse_config());
    let template = ok_or_exit(template_setup());
    let database = Arc::new(Mutex::new(ok_or_exit(db_connection(&config.db_path))));
    let listen_addr = (config.host.clone(), config.port);
    let app_state = AppState {
        config,
        template,
        database,
    };
    let app = Router::new()
        .route("/", get(forum::show_posts))
        .route("/post", get(forum::show_create_post))
        .route("/post", post(forum::handle_create_post))
        .route("/post/{post_id}", get(forum::show_post))
        .route("/reply/{post_id}", get(forum::show_create_reply))
        .route("/reply/{post_id}", post(forum::handle_create_reply))
        .route("/delete/{post_id}", post(forum::handle_delete_post))
        .route("/assets/base.css", get(forum::base_css))
        .route("/assets/milligram.css", get(forum::milligram_css))
        .with_state(app_state);

    tracing::info!("Listening on http://{}:{}", listen_addr.0, listen_addr.1);
    let listener = ok_or_exit(tokio::net::TcpListener::bind(listen_addr).await);

    ok_or_exit(axum::serve(listener, app).await)
}

fn setup_tracing() {
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
}

fn parse_config() -> ForumResult<AppConfig> {
    let config = AppConfig::parse().map_err(ForumError::ConfigError)?;
    tracing::info!(config = ?config, "Parsed config");
    Ok(config)
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

fn ok_or_exit<T, E: std::fmt::Display>(result: Result<T, E>) -> T {
    match result {
        Ok(v) => v,
        Err(why) => {
            tracing::error!("{}", why);
            std::process::exit(1);
        }
    }
}
