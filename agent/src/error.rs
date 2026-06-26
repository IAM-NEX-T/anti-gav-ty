use thiserror::Error;

#[derive(Error, Debug)]
pub enum AgentError {
    #[error("HTTP error: {0}")]
    Http(#[from] reqwest::Error),

    #[error("config error: {0}")]
    Config(#[from] crate::config::ConfigError),

    #[error("registration failed: {0}")]
    Registration(String),

    #[error("heartbeat failed: {0}")]
    Heartbeat(String),
}
