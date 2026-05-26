use rusqlite::types::Value;

use crate::storage::{Result, StorageError};

#[derive(Debug, Default, PartialEq, Eq)]
struct ParsedLocalFilter {
    authors: Vec<String>,
    assignees: Vec<String>,
    labels: Vec<String>,
    repos: Vec<String>,
    review_requested: Vec<String>,
    reviewed_by: Vec<String>,
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

    if !parsed.authors.is_empty() {
        clauses.push(or_equals_clause(
            "lower(i.author_login)",
            &parsed.authors,
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

    Ok(Some(CompiledLocalFilter {
        clause: clauses.join(" AND "),
        params,
    }))
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
            "label" => parsed.labels.push(value),
            "repo" => parsed.repos.push(value),
            "review-requested" => parsed.review_requested.push(value),
            "reviewed-by" => parsed.reviewed_by.push(value),
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
    let placeholders = values
        .iter()
        .map(|value| {
            params.push(Value::Text(value.clone()));
            "?".to_owned()
        })
        .collect::<Vec<_>>()
        .join(", ");
    format!("{column} IN ({placeholders})")
}

fn or_relation_clause(
    table: &str,
    column: &str,
    values: &[String],
    params: &mut Vec<Value>,
) -> String {
    let placeholders = values
        .iter()
        .map(|value| {
            params.push(Value::Text(value.clone()));
            "?".to_owned()
        })
        .collect::<Vec<_>>()
        .join(", ");
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

#[cfg(test)]
mod tests {
    use rusqlite::types::Value;

    use super::compile;

    #[test]
    fn compiles_supported_local_filter_subset() {
        let compiled = compile(Some(
            r#"author:octo assignee:dev label:bug label:"needs triage" repo:acme/api review-requested:triage reviewed-by:reviewer"#,
        ))
        .expect("local filter should compile")
        .expect("compiled filter");

        assert!(compiled.clause.contains("lower(i.author_login) IN (?)"));
        assert!(compiled.clause.contains("FROM stream_item_assignees"));
        assert!(compiled.clause.contains("FROM stream_item_labels"));
        assert!(compiled
            .clause
            .contains("lower(i.repository_owner || '/' || i.repository_name) IN (?)"));
        assert!(compiled.clause.contains("FROM stream_item_review_requests"));
        assert!(compiled.clause.contains("FROM stream_item_reviews"));
        assert_eq!(
            compiled.params,
            vec![
                Value::Text("octo".to_owned()),
                Value::Text("acme/api".to_owned()),
                Value::Text("dev".to_owned()),
                Value::Text("triage".to_owned()),
                Value::Text("reviewer".to_owned()),
                Value::Text("bug".to_owned()),
                Value::Text("needs triage".to_owned()),
            ]
        );
    }

    #[test]
    fn rejects_unsupported_terms() {
        let error = compile(Some("milestone:v1"))
            .expect_err("unsupported local filter should fail")
            .to_string();
        assert!(error.contains("Unsupported local filter key"));
    }
}
