use crate::github::GitHubError;
use crate::models::HostConfig;

const USER_AGENT: &str = concat!(env!("CARGO_PKG_NAME"), "/", env!("CARGO_PKG_VERSION"));

pub(super) fn authenticated_get(
    host: &HostConfig,
    pat: &str,
    endpoint: &str,
) -> Result<ureq::http::Response<ureq::Body>, GitHubError> {
    let authorization = format!("Bearer {pat}");
    ureq::get(endpoint)
        .header("Accept", "application/vnd.github+json")
        .header("User-Agent", USER_AGENT)
        .header("Authorization", &authorization)
        .call()
        .map_err(|error| request_error(host, pat, error))
}

pub(super) fn authenticated_post_json(
    host: &HostConfig,
    pat: &str,
    endpoint: &str,
    body: String,
) -> Result<ureq::http::Response<ureq::Body>, GitHubError> {
    let authorization = format!("Bearer {pat}");
    ureq::post(endpoint)
        .header("Accept", "application/vnd.github+json")
        .header("Content-Type", "application/json")
        .header("User-Agent", USER_AGENT)
        .header("Authorization", &authorization)
        .send(body)
        .map_err(|error| request_error(host, pat, error))
}

pub(super) fn ensure_success(
    host: &HostConfig,
    response: &ureq::http::Response<ureq::Body>,
) -> Result<(), GitHubError> {
    let status = response.status();
    if status.as_u16() == 401 || status.as_u16() == 403 {
        return Err(GitHubError::Authentication {
            host: host.name.clone(),
        });
    }
    if !status.is_success() {
        return Err(GitHubError::Api {
            host: host.name.clone(),
            status: status.as_u16(),
        });
    }
    Ok(())
}

pub(super) fn read_body(
    host: &HostConfig,
    pat: &str,
    response: &mut ureq::http::Response<ureq::Body>,
) -> Result<String, GitHubError> {
    response
        .body_mut()
        .read_to_string()
        .map_err(|error| GitHubError::Network {
            host: host.name.clone(),
            message: sanitize_error_message(&error.to_string(), pat),
        })
}

pub(super) fn sanitize_error_message(message: &str, pat: &str) -> String {
    if pat.is_empty() {
        message.to_owned()
    } else {
        message.replace(pat, "<redacted>")
    }
}

fn request_error(host: &HostConfig, pat: &str, error: ureq::Error) -> GitHubError {
    GitHubError::Network {
        host: host.name.clone(),
        message: sanitize_error_message(&error.to_string(), pat),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sanitized_errors_do_not_include_pat() {
        assert_eq!(
            sanitize_error_message("failed with ghp_secret", "ghp_secret"),
            "failed with <redacted>"
        );
    }
}
