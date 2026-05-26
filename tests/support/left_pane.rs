use ghtl::app::screens::stream::{StreamEvent, StreamState};
use ghtl::models::{LibraryCounts, SavedQuery};

pub struct LeftPaneHarness {
    pub stream: StreamState,
    pub library_counts: LibraryCounts,
    pub saved_queries: Vec<SavedQuery>,
    pub event: Option<StreamEvent>,
}

pub struct StreamHarness {
    pub stream: StreamState,
    pub saved_queries: Vec<SavedQuery>,
    pub event: Option<StreamEvent>,
}

pub fn sample_saved_query() -> SavedQuery {
    SavedQuery {
        id: 7,
        name: "Reviews".to_owned(),
        query: "is:pr review-requested:@me".to_owned(),
        enabled: true,
        position: 0,
        unread_count: 3,
    }
}
