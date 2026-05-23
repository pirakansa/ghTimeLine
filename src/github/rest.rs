use crate::github::GitHubError;
use crate::models::{HostConfig, SortOrder};

const USER_AGENT: &str = concat!(env!("CARGO_PKG_NAME"), "/", env!("CARGO_PKG_VERSION"));

pub fn search_sort_query(sort: SortOrder) -> (&'static str, &'static str) {
    match sort {
        SortOrder::UpdatedDesc => ("updated", "desc"),
        SortOrder::UpdatedAsc => ("updated", "asc"),
        SortOrder::CreatedDesc => ("created", "desc"),
        SortOrder::CreatedAsc => ("created", "asc"),
        SortOrder::CommentsDesc => ("comments", "desc"),
        SortOrder::CommentsAsc => ("comments", "asc"),
    }
}

pub fn test_connection(host: &HostConfig, pat: &str) -> Result<(), GitHubError> {
    let endpoint = api_url(host, "user");
    let host_name = host.name.clone();
    let authorization = format!("Bearer {pat}");
    let response = ureq::get(&endpoint)
        .header("Accept", "application/vnd.github+json")
        .header("User-Agent", USER_AGENT)
        .header("Authorization", &authorization)
        .call();

    match response {
        Ok(response) if response.status().is_success() => Ok(()),
        Ok(response) if response.status().as_u16() == 401 || response.status().as_u16() == 403 => {
            Err(GitHubError::Authentication { host: host_name })
        }
        Ok(response) => Err(GitHubError::Api {
            host: host_name,
            status: response.status().as_u16(),
        }),
        Err(error) => Err(GitHubError::Network {
            host: host_name,
            message: sanitize_error_message(&error.to_string(), pat),
        }),
    }
}

pub fn api_url(host: &HostConfig, path: &str) -> String {
    let base = host.rest_api_base_url();
    let path = path.trim_start_matches('/');
    format!("{base}{path}")
}

fn sanitize_error_message(message: &str, pat: &str) -> String {
    if pat.is_empty() {
        message.to_owned()
    } else {
        message.replace(pat, "<redacted>")
    }
}

#[cfg(test)]
mod tests {
    use crate::models::{AppConfig, HostKind};

    use super::*;

    #[test]
    fn api_url_uses_normalized_rest_base_path() {
        let mut config = AppConfig::default_with_pat("token".to_owned());
        config.host.kind = HostKind::Ghes;
        config.host.hostname = "ghe.example.test".to_owned();
        config.host.rest_api_base_path = "/api/v3/".to_owned();

        assert_eq!(
            api_url(&config.host, "/user"),
            "https://ghe.example.test/api/v3/user"
        );
    }

    #[test]
    fn sanitized_errors_do_not_include_pat() {
        assert_eq!(
            sanitize_error_message("failed with ghp_secret", "ghp_secret"),
            "failed with <redacted>"
        );
    }
}
