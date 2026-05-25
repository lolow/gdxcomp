/// Errors raised by the gdxcomp core.
#[derive(Debug, thiserror::Error)]
pub enum CoreError {
    #[error(transparent)]
    Gdx(#[from] gdx::GdxError),

    #[error("symbol {0:?} is not present in any selected file")]
    SymbolMissing(String),

    #[error("dimension index {dim} is out of range for symbol {symbol:?} (dim {ndim})")]
    DimOutOfRange {
        symbol: String,
        dim: usize,
        ndim: usize,
    },

    #[error("invalid display setup: {0}")]
    InvalidSetup(String),

    #[error("failed to (de)serialize display setup: {0}")]
    Json(#[from] serde_json::Error),
}

pub type Result<T> = std::result::Result<T, CoreError>;
