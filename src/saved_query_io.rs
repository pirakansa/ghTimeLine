use std::collections::HashSet;
use std::fs;
use std::path::Path;

use serde::{Deserialize, Serialize};
use thiserror::Error;

use crate::config;
use crate::models::{HostConfig, SavedQuery};

const SAVED_QUERY_DOCUMENT_VERSION: u32 = 1;

#[derive(Debug, Error)]
pub enum SavedQueryIoError {
    #[error("saved query file could not be read: {0}")]
    Read(#[source] std::io::Error),
    #[error("saved query file could not be written: {0}")]
    Write(#[source] std::io::Error),
    #[error("saved query file is not valid YAML: {0}")]
    Parse(#[from] serde_yaml::Error),
    #[error("saved query file is invalid: {0}")]
    Validation(String),
}

pub type Result<T> = std::result::Result<T, SavedQueryIoError>;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ImportedSavedQueries {
    pub host: HostConfig,
    pub queries: Vec<ImportedSavedQuery>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ImportedSavedQuery {
    pub name: String,
    pub query: String,
    pub enabled: bool,
    pub position: i64,
    pub filter_streams: Vec<ImportedFilterStream>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ImportedFilterStream {
    pub name: String,
    pub filter_query: String,
    pub enabled: bool,
    pub position: i64,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
struct SavedQueryDocument {
    version: u32,
    host: HostConfig,
    queries: Vec<SavedQueryDocumentEntry>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
struct SavedQueryDocumentEntry {
    name: String,
    query: String,
    enabled: bool,
    position: i64,
    #[serde(default)]
    filter_streams: Vec<SavedFilterStreamDocumentEntry>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
struct SavedFilterStreamDocumentEntry {
    name: String,
    filter_query: String,
    enabled: bool,
    position: i64,
}

pub fn write_saved_queries(path: &Path, host: &HostConfig, queries: &[SavedQuery]) -> Result<()> {
    let document = SavedQueryDocument {
        version: SAVED_QUERY_DOCUMENT_VERSION,
        host: host.clone(),
        queries: queries
            .iter()
            .map(|query| SavedQueryDocumentEntry {
                name: query.name.clone(),
                query: query.query.clone(),
                enabled: query.enabled,
                position: query.position,
                filter_streams: query
                    .filter_streams
                    .iter()
                    .map(|filter_stream| SavedFilterStreamDocumentEntry {
                        name: filter_stream.name.clone(),
                        filter_query: filter_stream.filter_query.clone(),
                        enabled: filter_stream.enabled,
                        position: filter_stream.position,
                    })
                    .collect(),
            })
            .collect(),
    };

    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(SavedQueryIoError::Write)?;
    }

    let yaml = serde_yaml::to_string(&document)?;
    fs::write(path, yaml).map_err(SavedQueryIoError::Write)?;
    Ok(())
}

pub fn read_saved_queries(path: &Path) -> Result<ImportedSavedQueries> {
    let content = fs::read_to_string(path).map_err(SavedQueryIoError::Read)?;
    let document = serde_yaml::from_str::<SavedQueryDocument>(&content)?;
    validate_document(document)
}

fn validate_document(document: SavedQueryDocument) -> Result<ImportedSavedQueries> {
    if document.version != SAVED_QUERY_DOCUMENT_VERSION {
        return Err(SavedQueryIoError::Validation(format!(
            "unsupported saved query file version {}",
            document.version
        )));
    }

    let host = config::validate_host_config(document.host)
        .map_err(|err| SavedQueryIoError::Validation(err.to_string()))?;
    let mut seen_names = HashSet::new();
    let mut entries = document.queries;
    entries.sort_by(|left, right| {
        left.position
            .cmp(&right.position)
            .then_with(|| left.name.cmp(&right.name))
    });

    let queries = entries
        .into_iter()
        .enumerate()
        .map(|(index, entry)| {
            let name = trim_required("queries[].name", &entry.name)?;
            let query = trim_required("queries[].query", &entry.query)?;
            if !seen_names.insert(name.clone()) {
                return Err(SavedQueryIoError::Validation(format!(
                    "duplicate saved query name: {name}"
                )));
            }
            let filter_streams = normalize_filter_streams(entry.filter_streams)?;

            Ok(ImportedSavedQuery {
                name,
                query,
                enabled: entry.enabled,
                position: index as i64,
                filter_streams,
            })
        })
        .collect::<Result<Vec<_>>>()?;

    Ok(ImportedSavedQueries { host, queries })
}

fn normalize_filter_streams(
    entries: Vec<SavedFilterStreamDocumentEntry>,
) -> Result<Vec<ImportedFilterStream>> {
    let mut seen_names = HashSet::new();
    let mut entries = entries;
    entries.sort_by(|left, right| {
        left.position
            .cmp(&right.position)
            .then_with(|| left.name.cmp(&right.name))
    });

    entries
        .into_iter()
        .enumerate()
        .map(|(index, entry)| {
            let name = trim_required("queries[].filter_streams[].name", &entry.name)?;
            let filter_query = trim_required(
                "queries[].filter_streams[].filter_query",
                &entry.filter_query,
            )?;
            if !seen_names.insert(name.clone()) {
                return Err(SavedQueryIoError::Validation(format!(
                    "duplicate filter stream name in saved query: {name}"
                )));
            }

            Ok(ImportedFilterStream {
                name,
                filter_query,
                enabled: entry.enabled,
                position: index as i64,
            })
        })
        .collect()
}

fn trim_required(path: &str, value: &str) -> Result<String> {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        Err(SavedQueryIoError::Validation(format!(
            "{path} must not be empty"
        )))
    } else {
        Ok(trimmed.to_owned())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::AppConfig;

    #[test]
    fn read_saved_queries_normalizes_host_and_positions() {
        let yaml = r#"
version: 1
host:
  name: GitHub.com
  scheme: https
  hostname: api.github.com
  rest_api_base_path: api
  kind: github
queries:
  - name: " Review requested "
    query: " is:pr review-requested:@me "
    enabled: true
    position: 3
    filter_streams:
      - name: " Assigned to me "
        filter_query: " assignee:@me "
        enabled: true
        position: 4
      - name: Team mentions
        filter_query: " mentions:my-org "
        enabled: false
        position: 2
  - name: Mine
    query: "is:pr author:@me"
    enabled: false
    position: 1
"#;

        let imported =
            validate_document(serde_yaml::from_str(yaml).expect("yaml should deserialize"))
                .expect("document should validate");

        let mut expected_host = AppConfig::default_with_pat("token".to_owned()).host;
        expected_host.rest_api_base_path = "/api/".to_owned();
        assert_eq!(imported.host, expected_host);
        assert_eq!(imported.queries[0].name, "Mine");
        assert_eq!(imported.queries[0].position, 0);
        assert_eq!(imported.queries[1].name, "Review requested");
        assert_eq!(imported.queries[1].query, "is:pr review-requested:@me");
        assert_eq!(imported.queries[1].position, 1);
        assert_eq!(imported.queries[1].filter_streams[0].name, "Team mentions");
        assert_eq!(imported.queries[1].filter_streams[0].position, 0);
        assert_eq!(imported.queries[1].filter_streams[1].name, "Assigned to me");
        assert_eq!(
            imported.queries[1].filter_streams[1].filter_query,
            "assignee:@me"
        );
        assert_eq!(imported.queries[1].filter_streams[1].position, 1);
    }

    #[test]
    fn read_saved_queries_rejects_duplicate_names() {
        let yaml = r#"
version: 1
host:
  name: GitHub.com
  scheme: https
  hostname: api.github.com
  rest_api_base_path: /
  kind: github
queries:
  - name: Mine
    query: "is:pr"
    enabled: true
    position: 0
  - name: " Mine "
    query: "is:issue"
    enabled: true
    position: 1
"#;

        let err = validate_document(serde_yaml::from_str(yaml).expect("yaml should deserialize"))
            .expect_err("duplicate names must fail");

        assert!(err.to_string().contains("duplicate saved query name"));
    }

    #[test]
    fn read_saved_queries_rejects_duplicate_filter_stream_names() {
        let yaml = r#"
version: 1
host:
  name: GitHub.com
  scheme: https
  hostname: api.github.com
  rest_api_base_path: /
  kind: github
queries:
  - name: Mine
    query: "is:pr"
    enabled: true
    position: 0
    filter_streams:
      - name: Assigned
        filter_query: "assignee:@me"
        enabled: true
        position: 0
      - name: " Assigned "
        filter_query: "mentions:@me"
        enabled: true
        position: 1
"#;

        let err = validate_document(serde_yaml::from_str(yaml).expect("yaml should deserialize"))
            .expect_err("duplicate filter stream names must fail");

        assert!(err
            .to_string()
            .contains("duplicate filter stream name in saved query"));
    }
}
