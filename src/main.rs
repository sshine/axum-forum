use std::sync::{Arc, Mutex};

use axum::{
    routing::{get, post},
    Router,
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
    let template = template_setup().expect("Valid templates");
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

    let forum_hostname: String = env_var_default("FORUM_HOST", "127.0.0.1".to_string())
        .expect("A valid FORUM_HOST environment variable");
    let forum_port: u16 =
        env_var_parse_default("FORUM_PORT", 3000).expect("A valid FORUM_PORT environment variable");

    let addr = format!("{}:{}", forum_hostname, forum_port);

    let listener = tokio::net::TcpListener::bind(&addr)
        .await
        .expect("TcpListener::bind");

    println!("Listening on http://{}", addr);
    axum::serve(listener, app).await.expect("axum::serve");
}

fn db_connection() -> rusqlite::Connection {
    let forum_db_path = env_var_default("FORUM_DB_PATH", "forum.db".to_string())
        .expect("A valid FORUM_DB_PATH environment variable");

    let conn = rusqlite::Connection::open(&forum_db_path).expect("A database connection");

    if let Err(why) = conn
        .execute_batch(forum::FORUM_POSTS_SQL)
        .map_err(ForumError::DatabaseError)
    {
        panic!("Could not initialize database: {}", why);
    }

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
