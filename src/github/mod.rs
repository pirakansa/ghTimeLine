mod client;
pub mod discussion;
pub mod graphql;
pub mod project;
pub mod rest;

use thiserror::Error;

use crate::models::{AppConfig, HostConfig};

#[derive(Debug, Error)]
pub enum GitHubError {
    #[error("authentication failed for {host}")]
    Authentication { host: String },
    #[error("GitHub API returned HTTP {status} for {host}")]
    Api { host: String, status: u16 },
    #[error("network connection failed for {host}: {message}")]
    Network { host: String, message: String },
    #[error("GitHub API response could not be parsed for {host}: {message}")]
    Parse { host: String, message: String },
    #[error("GitHub API execution is not implemented yet")]
    NotImplemented,
}

pub struct GitHubService {
    host: HostConfig,
}

impl GitHubService {
    pub fn new(host: HostConfig) -> Self {
        Self { host }
    }

    pub fn host(&self) -> &HostConfig {
        &self.host
    }
}

pub fn test_connection(config: &AppConfig) -> Result<(), GitHubError> {
    rest::test_connection(&config.host, &config.auth.pat)
}
