use ghtl::models::AppConfig;
use ghtl::storage::Storage;
use ghtl::APP_TITLE;

#[test]
fn it_exposes_the_window_title() {
    assert_eq!(APP_TITLE, "ghTimeLine");
}

#[test]
fn storage_initializes_host_without_storing_pat() {
    let storage = Storage::in_memory().expect("storage should initialize");
    let config = AppConfig::default_with_pat("ghp_example".to_owned());

    let host_id = storage
        .ensure_host(&config.host)
        .expect("host should be inserted");

    let pat_count: i64 = storage
        .connection()
        .query_row(
            "SELECT COUNT(*) FROM hosts WHERE fingerprint LIKE '%ghp_example%'",
            [],
            |row| row.get(0),
        )
        .expect("query should run");

    assert!(host_id > 0);
    assert_eq!(pat_count, 0);
}

#[test]
fn saved_queries_schema_does_not_store_view_sort() {
    let storage = Storage::in_memory().expect("storage should initialize");
    let mut statement = storage
        .connection()
        .prepare("PRAGMA table_info(saved_queries)")
        .expect("schema query should prepare");
    let columns = statement
        .query_map([], |row| row.get::<_, String>(1))
        .expect("schema query should run")
        .collect::<Result<Vec<_>, _>>()
        .expect("schema columns should read");

    assert!(!columns.iter().any(|column| column == "sort"));
}
