#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ReviewSignal {
    None,
    ReviewRequired,
    ChangesRequested,
    Approved,
    Unknown,
}

impl ReviewSignal {
    pub fn as_db_value(&self) -> &'static str {
        match self {
            Self::None => "none",
            Self::ReviewRequired => "review_required",
            Self::ChangesRequested => "changes_requested",
            Self::Approved => "approved",
            Self::Unknown => "unknown",
        }
    }
}
