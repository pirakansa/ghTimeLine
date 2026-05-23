use gh_stream_listner::models::AppConfig;
use gh_stream_listner::storage::Storage;
use gh_stream_listner::APP_TITLE;

#[test]
fn it_exposes_the_window_title() {
    assert_eq!(APP_TITLE, "ghStreamListner");
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
