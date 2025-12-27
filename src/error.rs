use thiserror::Error;

#[derive(Debug, Error)]
pub enum Error {
    #[error("Failed to parse KDL: {0}")]
    Parse(#[from] kdl::KdlError),

    #[error("Schema error: {0}")]
    Schema(String),

    #[error("Validation error: {0}")]
    Validation(String),

    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
}
