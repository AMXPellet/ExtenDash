use thiserror::Error;

pub type Result<T, E = CoreError> = std::result::Result<T, E>;

/// Typed core errors. Frontends match on variants; never stringly-typed.
///
/// More variants (`Offline`, `AuthExpired`, `InvalidExtensionData`, ...) are
/// added by the milestones that introduce them.
#[derive(Debug, Error)]
#[non_exhaustive]
pub enum CoreError {
    #[error("unsupported schemaVersion {found} (this build supports {supported})")]
    SchemaMismatch { found: u32, supported: u32 },

    #[error("invalid {kind} id {value:?}: {reason}")]
    InvalidId {
        kind: &'static str,
        value: String,
        reason: &'static str,
    },

    #[error("invalid inbox file name {0:?}")]
    InvalidInboxFileName(String),

    #[error("invalid data: {0}")]
    InvalidData(#[from] serde_json::Error),
}
