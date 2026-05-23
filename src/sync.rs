use thiserror::Error;

#[derive(Debug, Error)]
pub enum SyncError {
    #[error("GitHub refresh is not implemented yet")]
    NotImplemented,
}
