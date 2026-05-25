use std::path::PathBuf;

/// Errors raised by the GDX wrapper.
#[derive(Debug, thiserror::Error)]
pub enum GdxError {
    #[error("failed to create GDX object: {0}")]
    Create(String),

    #[error("path contains an interior NUL byte: {0:?}")]
    InvalidPath(PathBuf),

    #[error("failed to open {path:?} for reading (gdx error {code}: {message})")]
    OpenRead {
        path: PathBuf,
        code: i32,
        message: String,
    },

    #[error("failed to open {path:?} for writing (gdx error {code}: {message})")]
    OpenWrite {
        path: PathBuf,
        code: i32,
        message: String,
    },

    #[error("symbol {0:?} not found")]
    SymbolNotFound(String),

    #[error("gdx operation {op} failed (error {code}: {message})")]
    Operation {
        op: &'static str,
        code: i32,
        message: String,
    },
}

pub type Result<T> = std::result::Result<T, GdxError>;
