pub mod graphql;
pub mod rest;

use thiserror::Error;

use crate::models::HostConfig;

#[derive(Debug, Error)]
pub enum GitHubError {
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
