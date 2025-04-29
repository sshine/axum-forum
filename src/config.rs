use config_manager::config;
use serde::{Deserialize, Serialize};

/// FIXME: Insert useful documentation that will show up in --help here.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[config(clap(version, author, long_about), env_prefix = "forum")]
pub struct AppConfig {
    #[source(env, config, default = "forum.db")]
    pub db_path: String,

    #[source(env, config, default = "127.0.0.1")]
    pub host: String,

    #[source(env, config, default = 3000)]
    pub port: u16,

    #[source(env, config, default = "Forum")]
    pub title: String,
}
