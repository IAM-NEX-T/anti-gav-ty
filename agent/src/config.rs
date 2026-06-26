use serde::Deserialize;
use std::fs;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum ConfigError {
    #[error("failed to read config file: {0}")]
    Read(#[from] std::io::Error),
    #[error("failed to parse config: {0}")]
    Parse(#[from] serde_yaml::Error),
}

#[derive(Debug, Deserialize, Clone)]
pub struct Config {
    pub agent: AgentConfig,
    pub backend: BackendConfig,
    #[serde(default)]
    pub nextdns: NextDnsConfig,
}

#[derive(Debug, Deserialize, Clone)]
pub struct AgentConfig {
    pub name: String,
    pub heartbeat_interval_secs: u64,
    #[allow(dead_code)]
    pub interface: Option<String>,
}

#[derive(Debug, Deserialize, Clone)]
pub struct BackendConfig {
    pub url: String,
    pub register_path: String,
    pub heartbeat_path: String,
}

#[derive(Debug, Deserialize, Clone, Default)]
pub struct NextDnsConfig {
    #[serde(default)]
    pub api_key: String,
    #[serde(default)]
    pub profile_id: String,
}

pub fn load(path: &str) -> Result<Config, ConfigError> {
    let contents = fs::read_to_string(path)?;
    let config: Config = serde_yaml::from_str(&contents)?;
    Ok(config)
}
