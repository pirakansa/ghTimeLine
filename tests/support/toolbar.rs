use ghtl::app::screens::stream::{StreamEvent, StreamState};
use ghtl::models::AppConfig;

pub struct ToolbarHarness {
    pub stream: StreamState,
    pub config: AppConfig,
    pub event: Option<StreamEvent>,
}

pub fn sample_toolbar_harness() -> ToolbarHarness {
    ToolbarHarness {
        stream: StreamState::default(),
        config: AppConfig::default_with_pat("ghp_test".to_owned()),
        event: None,
    }
}
