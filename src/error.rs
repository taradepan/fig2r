use thiserror::Error;

#[derive(Debug, Error)]
pub enum Fig2rError {
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),
    #[error("JSON error: {0}")]
    Json(#[from] serde_json::Error),
    #[error("{0}")]
    Message(String),
}

impl From<&str> for Fig2rError {
    fn from(value: &str) -> Self {
        Self::Message(value.to_string())
    }
}

impl From<String> for Fig2rError {
    fn from(value: String) -> Self {
        Self::Message(value)
    }
}
