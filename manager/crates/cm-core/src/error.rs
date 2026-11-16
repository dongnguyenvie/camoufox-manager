//! Typed failures. They travel inside `anyhow::Error`; `cm-api` downcasts them to pick the
//! HTTP status (anything else becomes 400).

use thiserror::Error;

#[derive(Debug, Error)]
pub enum CmError {
    /// The thing asked for does not exist (404).
    #[error("{0} not found")]
    NotFound(String),
    /// Bad input from the caller (400).
    #[error("{0}")]
    Invalid(String),
    /// The request clashes with the current state, e.g. profile is running (409).
    #[error("{0}")]
    Conflict(String),
    /// The area exists but its phase has not been implemented yet (501).
    #[error("{0}: not implemented yet")]
    NotImplemented(&'static str),
}

/// Shorthand used by stubs: `return Err(not_implemented("cookies"))`.
pub fn not_implemented(area: &'static str) -> anyhow::Error {
    CmError::NotImplemented(area).into()
}

/// `true` when `e` carries [`CmError::NotImplemented`].
pub fn is_not_implemented(e: &anyhow::Error) -> bool {
    matches!(e.downcast_ref::<CmError>(), Some(CmError::NotImplemented(_)))
}
