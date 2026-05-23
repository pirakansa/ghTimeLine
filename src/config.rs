use std::fs;
use std::path::{Path, PathBuf};

use thiserror::Error;

use crate::models::{AppConfig, HostKind};

const APP_DIR_NAME: &str = "ghstreamlistner";

#[derive(Debug, Error)]
pub enum ConfigError {
    #[error("configuration file could not be read: {0}")]
    Read(#[from] std::io::Error),
    #[error("configuration file is not valid YAML: {0}")]
    Parse(#[from] serde_yaml::Error),
    #[error("configuration is invalid: {0}")]
    Validation(String),
}

pub type Result<T> = std::result::Result<T, ConfigError>;

pub fn default_config_path() -> PathBuf {
    config_dir().join("config.yml")
}

pub fn default_database_path() -> PathBuf {
    data_dir().join("ghstreamlistner.db")
}

fn config_dir() -> PathBuf {
    if cfg!(target_os = "windows") {
        dirs_next::config_dir()
            .unwrap_or_else(|| PathBuf::from("."))
            .join(APP_DIR_NAME)
    } else {
        std::env::var_os("XDG_CONFIG_HOME")
            .map(PathBuf::from)
            .or_else(|| dirs_next::home_dir().map(|home| home.join(".config")))
            .unwrap_or_else(|| PathBuf::from("."))
            .join(APP_DIR_NAME)
    }
}

fn data_dir() -> PathBuf {
    if cfg!(target_os = "windows") {
        dirs_next::data_local_dir()
            .unwrap_or_else(|| PathBuf::from("."))
            .join(APP_DIR_NAME)
    } else {
        std::env::var_os("XDG_DATA_HOME")
            .map(PathBuf::from)
            .or_else(|| dirs_next::home_dir().map(|home| home.join(".local/share")))
            .unwrap_or_else(|| PathBuf::from("."))
            .join(APP_DIR_NAME)
    }
}

pub fn load_config(path: &Path) -> Result<AppConfig> {
    let content = fs::read_to_string(path)?;
    let config = serde_yaml::from_str::<AppConfig>(&content)?;
    validate_config(config)
}

pub fn write_config(path: &Path, config: &AppConfig) -> Result<()> {
    let normalized = validate_config(config.clone())?;
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    let content = serde_yaml::to_string(&normalized)?;
    fs::write(path, content)?;
    Ok(())
}

pub fn validate_config(mut config: AppConfig) -> Result<AppConfig> {
    config.host.name = trim_required("host.name", &config.host.name)?;
    config.host.hostname = validate_hostname(&config.host.hostname)?;
    config.host.rest_api_base_path = normalize_rest_api_base_path(&config.host.rest_api_base_path)?;
    config.auth.pat = trim_required("auth.pat", &config.auth.pat)?;

    if config.host.kind == HostKind::GitHub && config.host.hostname != "api.github.com" {
        return Err(ConfigError::Validation(
            "host.kind github requires host.hostname api.github.com".to_owned(),
        ));
    }

    if !is_hex_color(&config.ui.accent_color) {
        return Err(ConfigError::Validation(
            "ui.accent_color must be a #RRGGBB hex color".to_owned(),
        ));
    }

    if !(15..=3600).contains(&config.refresh.polling_interval_seconds) {
        return Err(ConfigError::Validation(
            "refresh.polling_interval_seconds must be between 15 and 3600".to_owned(),
        ));
    }

    Ok(config)
}

pub fn redact_pat(value: &str) -> String {
    if value.is_empty() {
        "<empty>".to_owned()
    } else {
        "<redacted>".to_owned()
    }
}

fn trim_required(path: &str, value: &str) -> Result<String> {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        Err(ConfigError::Validation(format!("{path} must not be empty")))
    } else {
        Ok(trimmed.to_owned())
    }
}

fn validate_hostname(value: &str) -> Result<String> {
    let trimmed = trim_required("host.hostname", value)?;
    let invalid = trimmed.contains("://")
        || trimmed.contains('/')
        || trimmed.contains('?')
        || trimmed.contains('#')
        || trimmed.contains('@')
        || trimmed.contains(':');
    if invalid {
        return Err(ConfigError::Validation(
            "host.hostname must be a hostname without scheme, path, query, fragment, username, password, or port".to_owned(),
        ));
    }
    Ok(trimmed)
}

fn normalize_rest_api_base_path(value: &str) -> Result<String> {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        return Err(ConfigError::Validation(
            "host.rest_api_base_path must not be empty".to_owned(),
        ));
    }
    let without_edges = trimmed.trim_matches('/');
    if without_edges.is_empty() {
        Ok("/".to_owned())
    } else {
        Ok(format!("/{without_edges}/"))
    }
}

fn is_hex_color(value: &str) -> bool {
    let bytes = value.as_bytes();
    bytes.len() == 7 && bytes[0] == b'#' && bytes[1..].iter().all(|byte| byte.is_ascii_hexdigit())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::{AppConfig, HostKind};

    #[test]
    fn normalizes_rest_base_path() {
        let mut config = AppConfig::default_with_pat("token".to_owned());
        config.host.kind = HostKind::Ghes;
        config.host.hostname = "ghe.example.test".to_owned();
        config.host.rest_api_base_path = "api/v3".to_owned();

        let normalized = validate_config(config).expect("config should be valid");

        assert_eq!(normalized.host.rest_api_base_path, "/api/v3/");
        assert_eq!(
            normalized.host.fingerprint(),
            "ghes|https|ghe.example.test|/api/v3/"
        );
    }

    #[test]
    fn rejects_patless_config() {
        let err = validate_config(AppConfig::default_with_pat(" ".to_owned()))
            .expect_err("missing PAT must be rejected");

        assert!(err.to_string().contains("auth.pat"));
    }
}
