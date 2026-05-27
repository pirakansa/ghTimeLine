use rusqlite::types::Value;

use crate::storage::{Result, StorageError};

#[derive(Debug, Default, PartialEq, Eq)]
struct ParsedLocalFilter {
    authors: Vec<String>,
    assignees: Vec<String>,
    involves: Vec<String>,
    item_types: Vec<String>,
    item_states: Vec<IsState>,
    labels: Vec<String>,
    org_owners: Vec<String>,
    repos: Vec<String>,
    review_requested: Vec<String>,
    reviewed_by: Vec<String>,
    user_owners: Vec<String>,
    draft_values: Vec<bool>,
}

#[derive(Debug, Default, PartialEq)]
pub(super) struct CompiledLocalFilter {
    pub clause: String,
    pub params: Vec<Value>,
}

pub(super) fn compile(query: Option<&str>) -> Result<Option<CompiledLocalFilter>> {
    let Some(query) = query.map(str::trim).filter(|query| !query.is_empty()) else {
        return Ok(None);
    };

    let parsed = parse(query)?;
    let mut clauses = Vec::new();
    let mut params = Vec::new();

    if !parsed.item_types.is_empty() {
        clauses.push(or_equals_clause(
            "i.item_type",
            &parsed.item_types,
            &mut params,
        ));
    }
    if !parsed.item_states.is_empty() {
        clauses.push(or_is_state_clause(&parsed.item_states));
    }
    if !parsed.authors.is_empty() {
        clauses.push(or_equals_clause(
            "lower(i.author_login)",
            &parsed.authors,
            &mut params,
        ));
    }
    if !parsed.user_owners.is_empty() {
        clauses.push(or_equals_clause(
            "lower(i.repository_owner)",
            &parsed.user_owners,
            &mut params,
        ));
    }
    if !parsed.org_owners.is_empty() {
        clauses.push(or_equals_clause(
            "lower(i.repository_owner)",
            &parsed.org_owners,
            &mut params,
        ));
    }
    if !parsed.repos.is_empty() {
        clauses.push(or_equals_clause(
            "lower(i.repository_owner || '/' || i.repository_name)",
            &parsed.repos,
            &mut params,
        ));
    }
    if !parsed.assignees.is_empty() {
        clauses.push(or_relation_clause(
            "stream_item_assignees",
            "login",
            &parsed.assignees,
            &mut params,
        ));
    }
    if !parsed.involves.is_empty() {
        clauses.push(or_involves_clause(&parsed.involves, &mut params));
    }
    if !parsed.review_requested.is_empty() {
        clauses.push(or_relation_clause(
            "stream_item_review_requests",
            "login",
            &parsed.review_requested,
            &mut params,
        ));
    }
    if !parsed.reviewed_by.is_empty() {
        clauses.push(or_relation_clause(
            "stream_item_reviews",
            "login",
            &parsed.reviewed_by,
            &mut params,
        ));
    }
    if !parsed.labels.is_empty() {
        for label in parsed.labels {
            clauses.push(exists_relation_clause(
                "stream_item_labels",
                "name",
                &label,
                &mut params,
            ));
        }
    }
    if !parsed.draft_values.is_empty() {
        clauses.push(or_draft_clause(&parsed.draft_values));
    }

    Ok(Some(CompiledLocalFilter {
        clause: clauses.join(" AND "),
        params,
    }))
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum IsState {
    Open,
    Closed,
    Merged,
}

fn parse(query: &str) -> Result<ParsedLocalFilter> {
    let mut parsed = ParsedLocalFilter::default();

    for token in tokenize(query)? {
        let Some((key, value)) = token.split_once(':') else {
            return Err(StorageError::InvalidFilter(format!(
                "Unsupported local filter term: {token}"
            )));
        };

        let key = key.trim().to_ascii_lowercase();
        let value = value.trim().to_ascii_lowercase();
        if value.is_empty() {
            return Err(StorageError::InvalidFilter(format!(
                "Local filter value must not be empty: {token}"
            )));
        }

        match key.as_str() {
            "author" => parsed.authors.push(value),
            "assignee" => parsed.assignees.push(value),
            "draft" => parsed.draft_values.push(parse_bool_filter(&key, &value)?),
            "involves" => parsed.involves.push(value),
            "is" => match value.as_str() {
                "issue" => parsed.item_types.push("issue".to_owned()),
                "pr" => parsed.item_types.push("pull_request".to_owned()),
                "open" => parsed.item_states.push(IsState::Open),
                "closed" => parsed.item_states.push(IsState::Closed),
                "merged" => parsed.item_states.push(IsState::Merged),
                _ => {
                    return Err(StorageError::InvalidFilter(format!(
                            "Unsupported value for 'is' filter: {value} (expected 'issue', 'pr', 'open', 'closed', or 'merged')"
                        )));
                }
            },
            "label" => parsed.labels.push(value),
            "org" => parsed.org_owners.push(value),
            "repo" => parsed.repos.push(value),
            "review-requested" => parsed.review_requested.push(value),
            "reviewed-by" => parsed.reviewed_by.push(value),
            "user" => parsed.user_owners.push(value),
            _ => {
                return Err(StorageError::InvalidFilter(format!(
                    "Unsupported local filter key: {key}"
                )));
            }
        }
    }

    Ok(parsed)
}

fn tokenize(query: &str) -> Result<Vec<String>> {
    let mut tokens = Vec::new();
    let mut current = String::new();
    let mut in_quotes = false;

    for ch in query.trim().chars() {
        match ch {
            '"' => in_quotes = !in_quotes,
            ' ' if !in_quotes => {
                if !current.is_empty() {
                    tokens.push(std::mem::take(&mut current));
                }
            }
            _ => current.push(ch),
        }
    }

    if in_quotes {
        return Err(StorageError::InvalidFilter(
            "Local filter contains an unterminated quote.".to_owned(),
        ));
    }

    if !current.is_empty() {
        tokens.push(current);
    }

    Ok(tokens)
}

fn or_equals_clause(column: &str, values: &[String], params: &mut Vec<Value>) -> String {
    let placeholders = push_text_placeholders(values, params);
    format!("{column} IN ({placeholders})")
}

fn or_is_state_clause(values: &[IsState]) -> String {
    let clauses = values
        .iter()
        .map(|value| match value {
            IsState::Open => "i.state = 'open'".to_owned(),
            IsState::Closed => "(i.state = 'closed' AND COALESCE(i.is_merged, 0) = 0)".to_owned(),
            IsState::Merged => "COALESCE(i.is_merged, 0) = 1".to_owned(),
        })
        .collect::<Vec<_>>()
        .join(" OR ");
    format!("({clauses})")
}

fn or_draft_clause(values: &[bool]) -> String {
    let clauses = values
        .iter()
        .map(|value| {
            format!(
                "(i.item_type = 'pull_request' AND COALESCE(i.is_draft, 0) = {})",
                i64::from(*value)
            )
        })
        .collect::<Vec<_>>()
        .join(" OR ");
    format!("({clauses})")
}

fn parse_bool_filter(key: &str, value: &str) -> Result<bool> {
    match value {
        "true" => Ok(true),
        "false" => Ok(false),
        _ => Err(StorageError::InvalidFilter(format!(
            "Unsupported value for '{key}' filter: {value} (expected 'true' or 'false')"
        ))),
    }
}

fn or_relation_clause(
    table: &str,
    column: &str,
    values: &[String],
    params: &mut Vec<Value>,
) -> String {
    let placeholders = push_text_placeholders(values, params);
    format!(
        "EXISTS (
            SELECT 1
            FROM {table}
            WHERE {table}.stream_item_id = i.id
              AND lower({table}.{column}) IN ({placeholders})
        )"
    )
}

fn exists_relation_clause(
    table: &str,
    column: &str,
    value: &str,
    params: &mut Vec<Value>,
) -> String {
    params.push(Value::Text(value.to_owned()));
    format!(
        "EXISTS (
            SELECT 1
            FROM {table}
            WHERE {table}.stream_item_id = i.id
              AND lower({table}.{column}) = ?
        )"
    )
}

fn or_involves_clause(values: &[String], params: &mut Vec<Value>) -> String {
    let author_placeholders = push_text_placeholders(values, params);

    let assignee_clause = or_relation_clause("stream_item_assignees", "login", values, params);
    let review_requested_clause =
        or_relation_clause("stream_item_review_requests", "login", values, params);
    let reviewed_by_clause = or_relation_clause("stream_item_reviews", "login", values, params);
    let participant_clause =
        or_relation_clause("stream_item_participants", "login", values, params);
    let mentions_clause = or_relation_clause("stream_item_mentions", "login", values, params);

    format!(
        "(lower(i.author_login) IN ({author_placeholders})
          OR {assignee_clause}
          OR {review_requested_clause}
          OR {reviewed_by_clause}
          OR {participant_clause}
          OR {mentions_clause})"
    )
}

fn push_text_placeholders(values: &[String], params: &mut Vec<Value>) -> String {
    values
        .iter()
        .map(|value| {
            params.push(Value::Text(value.clone()));
            "?".to_owned()
        })
        .collect::<Vec<_>>()
        .join(", ")
}

#[cfg(test)]
mod tests {
    use rusqlite::types::Value;

    use super::compile;

    #[test]
    fn compiles_supported_local_filter_subset() {
        let compiled = compile(Some(
            r#"author:octo assignee:dev involves:octo is:issue is:open draft:true label:bug label:"needs triage" org:acme repo:acme/api review-requested:triage reviewed-by:reviewer user:acme"#,
        ))
        .expect("local filter should compile")
        .expect("compiled filter");

        assert!(compiled.clause.contains("i.state = 'open'"));
        assert!(compiled.clause.contains("lower(i.author_login) IN (?)"));
        assert!(compiled.clause.contains("FROM stream_item_assignees"));
        assert!(compiled.clause.contains("lower(i.author_login) IN (?)"));
        assert!(compiled.clause.contains("FROM stream_item_labels"));
        assert!(compiled.clause.contains("lower(i.repository_owner) IN (?)"));
        assert!(compiled
            .clause
            .contains("lower(i.repository_owner || '/' || i.repository_name) IN (?)"));
        assert!(compiled.clause.contains("FROM stream_item_review_requests"));
        assert!(compiled.clause.contains("FROM stream_item_reviews"));
        assert!(compiled.clause.contains("FROM stream_item_participants"));
        assert!(compiled.clause.contains("FROM stream_item_mentions"));
        assert!(compiled.clause.contains("i.item_type IN (?)"));
        assert!(compiled
            .clause
            .contains("i.item_type = 'pull_request' AND COALESCE(i.is_draft, 0) = 1"));
        assert_eq!(
            compiled.params,
            vec![
                Value::Text("issue".to_owned()),
                Value::Text("octo".to_owned()),
                Value::Text("acme".to_owned()),
                Value::Text("acme".to_owned()),
                Value::Text("acme/api".to_owned()),
                Value::Text("dev".to_owned()),
                Value::Text("octo".to_owned()),
                Value::Text("octo".to_owned()),
                Value::Text("octo".to_owned()),
                Value::Text("octo".to_owned()),
                Value::Text("octo".to_owned()),
                Value::Text("octo".to_owned()),
                Value::Text("triage".to_owned()),
                Value::Text("reviewer".to_owned()),
                Value::Text("bug".to_owned()),
                Value::Text("needs triage".to_owned()),
            ]
        );
    }

    #[test]
    fn compiles_is_pr_to_pull_request_type() {
        let compiled = compile(Some("is:pr"))
            .expect("is:pr should compile")
            .expect("compiled filter");

        assert!(compiled.clause.contains("i.item_type IN (?)"));
        assert_eq!(
            compiled.params,
            vec![Value::Text("pull_request".to_owned())]
        );
    }

    #[test]
    fn compiles_is_type_and_state_with_and_semantics() {
        let compiled = compile(Some("is:pr is:open"))
            .expect("is:pr is:open should compile")
            .expect("compiled filter");

        assert!(compiled
            .clause
            .contains("i.item_type IN (?) AND (i.state = 'open')"));
        assert_eq!(
            compiled.params,
            vec![Value::Text("pull_request".to_owned())]
        );
    }

    #[test]
    fn compiles_multiple_involves_values_for_each_supported_relationship() {
        let compiled = compile(Some("involves:octo involves:dev"))
            .expect("involves terms should compile")
            .expect("compiled filter");

        assert_eq!(compiled.clause.matches("IN (?, ?)").count(), 6);
        assert_eq!(
            compiled.params,
            ["octo", "dev"]
                .repeat(6)
                .into_iter()
                .map(|value| Value::Text(value.to_owned()))
                .collect::<Vec<_>>()
        );
    }

    #[test]
    fn rejects_invalid_is_value() {
        let error = compile(Some("is:read"))
            .expect_err("invalid is value should fail")
            .to_string();
        assert!(error.contains("Unsupported value for 'is' filter"));
    }

    #[test]
    fn rejects_invalid_draft_value() {
        let error = compile(Some("draft:maybe"))
            .expect_err("invalid draft value should fail")
            .to_string();
        assert!(error.contains("Unsupported value for 'draft' filter"));
    }

    #[test]
    fn rejects_unsupported_terms() {
        let error = compile(Some("milestone:v1"))
            .expect_err("unsupported local filter should fail")
            .to_string();
        assert!(error.contains("Unsupported local filter key"));
    }
}
