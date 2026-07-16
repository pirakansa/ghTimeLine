use std::fs::{self, OpenOptions};
use std::io::Write;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};

use thiserror::Error;

use crate::models::{AppConfig, HostKind};

const APP_DIR_NAME: &str = "ghtl";
static TEMP_FILE_SEQUENCE: AtomicU64 = AtomicU64::new(0);

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

pub fn default_saved_queries_path() -> PathBuf {
    config_dir().join("saved-queries.yml")
}

pub fn default_database_path() -> PathBuf {
    data_dir().join("ghtl.db")
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
    atomic_write(path, content.as_bytes())
}

fn atomic_write(path: &Path, content: &[u8]) -> Result<()> {
    atomic_write_with(path, content, platform_replace)
}

#[cfg(not(windows))]
fn platform_replace(from: &Path, to: &Path) -> std::io::Result<()> {
    fs::rename(from, to)
}

#[cfg(windows)]
fn platform_replace(from: &Path, to: &Path) -> std::io::Result<()> {
    use std::os::windows::ffi::OsStrExt;
    use windows_sys::Win32::Storage::FileSystem::{ReplaceFileW, REPLACEFILE_WRITE_THROUGH};

    if !to.exists() {
        return fs::rename(from, to);
    }

    let from = from
        .as_os_str()
        .encode_wide()
        .chain(std::iter::once(0))
        .collect::<Vec<_>>();
    let to = to
        .as_os_str()
        .encode_wide()
        .chain(std::iter::once(0))
        .collect::<Vec<_>>();
    // SAFETY: Both paths are NUL-terminated UTF-16 buffers that remain alive for the call,
    // and the optional backup/exclusion arguments are intentionally null.
    let replaced = unsafe {
        ReplaceFileW(
            to.as_ptr(),
            from.as_ptr(),
            std::ptr::null(),
            REPLACEFILE_WRITE_THROUGH,
            std::ptr::null_mut(),
            std::ptr::null_mut(),
        )
    };
    if replaced == 0 {
        Err(std::io::Error::last_os_error())
    } else {
        Ok(())
    }
}

fn atomic_write_with(
    path: &Path,
    content: &[u8],
    replace: impl FnOnce(&Path, &Path) -> std::io::Result<()>,
) -> Result<()> {
    let parent = path.parent().unwrap_or_else(|| Path::new("."));
    let file_name = path
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or("config.yml");
    let sequence = TEMP_FILE_SEQUENCE.fetch_add(1, Ordering::Relaxed);
    let temporary_path = parent.join(format!(
        ".{file_name}.{}.{sequence}.tmp",
        std::process::id()
    ));

    let result = (|| {
        let mut temporary = create_temporary_file(&temporary_path)?;
        temporary.write_all(content)?;
        temporary.sync_all()?;
        replace(&temporary_path, path)?;
        Ok(())
    })();

    if result.is_err() {
        let _ = fs::remove_file(&temporary_path);
    }
    result
}

#[cfg(unix)]
fn create_temporary_file(path: &Path) -> std::io::Result<fs::File> {
    use std::os::unix::fs::OpenOptionsExt;

    OpenOptions::new()
        .write(true)
        .create_new(true)
        .mode(0o600)
        .open(path)
}

#[cfg(not(unix))]
fn create_temporary_file(path: &Path) -> std::io::Result<fs::File> {
    OpenOptions::new().write(true).create_new(true).open(path)
}

pub fn validate_host_config(
    mut host: crate::models::HostConfig,
) -> Result<crate::models::HostConfig> {
    host.name = trim_required("host.name", &host.name)?;
    host.hostname = validate_hostname(&host.hostname)?;
    host.rest_api_base_path = normalize_rest_api_base_path(&host.rest_api_base_path)?;

    if host.kind == HostKind::GitHub && host.hostname != "api.github.com" {
        return Err(ConfigError::Validation(
            "host.kind github requires host.hostname api.github.com".to_owned(),
        ));
    }

    Ok(host)
}

pub fn validate_config(mut config: AppConfig) -> Result<AppConfig> {
    config.host = validate_host_config(config.host)?;
    config.auth.pat = trim_required("auth.pat", &config.auth.pat)?;

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
    use std::time::{SystemTime, UNIX_EPOCH};

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

    #[test]
    fn failed_atomic_write_preserves_existing_config() {
        let directory = std::env::temp_dir().join(format!(
            "ghtl-config-test-{}-{}",
            std::process::id(),
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .expect("system time")
                .as_nanos()
        ));
        fs::create_dir_all(&directory).expect("test directory");
        let path = directory.join("config.yml");
        fs::write(&path, "original").expect("existing config");

        let error = atomic_write_with(&path, b"replacement", |_, _| {
            Err(std::io::Error::new(
                std::io::ErrorKind::PermissionDenied,
                "injected replacement failure",
            ))
        })
        .expect_err("replacement must fail");

        assert_eq!(
            fs::read_to_string(&path).expect("existing config"),
            "original"
        );
        assert!(error.to_string().contains("injected replacement failure"));
        let temporary_files = fs::read_dir(&directory)
            .expect("directory entries")
            .filter_map(|entry| entry.ok())
            .filter(|entry| entry.file_name().to_string_lossy().ends_with(".tmp"))
            .count();
        assert_eq!(temporary_files, 0);
        fs::remove_dir_all(directory).expect("cleanup");
    }

    #[cfg(unix)]
    #[test]
    fn atomic_write_keeps_config_private() {
        use std::os::unix::fs::PermissionsExt;

        let directory = std::env::temp_dir().join(format!(
            "ghtl-config-permissions-test-{}-{}",
            std::process::id(),
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .expect("system time")
                .as_nanos()
        ));
        fs::create_dir_all(&directory).expect("test directory");
        let path = directory.join("config.yml");

        atomic_write(&path, b"secret").expect("atomic write");

        let mode = fs::metadata(&path)
            .expect("config metadata")
            .permissions()
            .mode();
        assert_eq!(mode & 0o777, 0o600);
        fs::remove_dir_all(directory).expect("cleanup");
    }
}
