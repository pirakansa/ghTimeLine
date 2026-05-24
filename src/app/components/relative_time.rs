use chrono::{DateTime, Utc};

/// Formats an ISO 8601 timestamp as a human-readable relative time string
/// (e.g. "3 hours ago"). Falls back to the original string on parse failure.
pub fn format(iso: &str) -> String {
    let Ok(dt) = iso.parse::<DateTime<Utc>>() else {
        return iso.to_string();
    };
    let now = Utc::now();
    let secs = (now - dt).num_seconds().max(0);

    if secs < 60 {
        "just now".to_string()
    } else if secs < 3600 {
        let m = secs / 60;
        format!("{m} min ago")
    } else if secs < 86_400 {
        let h = secs / 3600;
        format!("{h} hours ago")
    } else if secs < 86_400 * 30 {
        let d = secs / 86_400;
        format!("{d} days ago")
    } else if secs < 86_400 * 365 {
        let mo = secs / (86_400 * 30);
        format!("{mo} months ago")
    } else {
        let y = secs / (86_400 * 365);
        format!("{y} years ago")
    }
}

#[cfg(test)]
mod tests {
    use super::format;
    use chrono::{Duration, Utc};

    fn ago(secs: i64) -> String {
        let dt = Utc::now() - Duration::seconds(secs);
        format(&dt.to_rfc3339())
    }

    #[test]
    fn formats_relative_times() {
        assert_eq!(ago(30), "just now");
        assert_eq!(ago(90), "1 min ago");
        assert_eq!(ago(7200), "2 hours ago");
        assert_eq!(ago(86_400 * 3), "3 days ago");
        assert_eq!(ago(86_400 * 60), "2 months ago");
        assert_eq!(ago(86_400 * 400), "1 years ago");
    }

    #[test]
    fn falls_back_for_invalid_input() {
        assert_eq!(format("not-a-date"), "not-a-date");
    }
}
