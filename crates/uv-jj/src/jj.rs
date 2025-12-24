use std::path::PathBuf;
use std::sync::LazyLock;

#[derive(Debug, thiserror::Error)]
pub enum JjError {
    #[error("Jujutsu executable not found. Ensure that Jujutsu is installed and available.")]
    JjNotFound,
    #[error(transparent)]
    Other(#[from] which::Error),
}

/// A global cache of the result of `which jj`.
pub static JJ: LazyLock<Result<PathBuf, JjError>> = LazyLock::new(|| {
    which::which("jj").map_err(|err| match err {
        which::Error::CannotFindBinaryPath => JjError::JjNotFound,
        err => JjError::Other(err),
    })
});
