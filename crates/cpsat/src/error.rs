//! The crate's error type.

/// An error from the native bridge or from decoding its output.
///
/// An invalid model is not an error of a solve, which reports it as the
/// `MODEL_INVALID` status, but [`Model::validate`](crate::prelude::Model::validate)
/// returns it as [`Error::InvalidModel`].
#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
pub enum Error {
    /// OR-Tools or the bridge failed, for example by throwing a C++ exception.
    #[error("OR-Tools failed: {0}")]
    Native(String),
    /// OR-Tools' validation rejected the model.
    #[error("invalid model: {0}")]
    InvalidModel(String),
    /// The bridge returned bytes that are not a valid message.
    #[error("failed to decode the solver response")]
    Decode(#[from] buffa::DecodeError),
}

/// A result with this crate's [`Error`].
pub type Result<T, E = Error> = core::result::Result<T, E>;
