use serde::Serialize;
use specta::Type;
use thiserror::Error;

#[derive(Debug, Error, Serialize, Type)]
#[serde(tag = "type", content = "data", rename_all = "snake_case")]
pub enum CommandError {
    #[error("network error: {0}")]
    Network(String),

    #[error("authentication required")]
    NotAuthenticated,

    #[error("token expired")]
    TokenExpired,

    #[error("two-factor authentication required")]
    Requires2fa,

    #[error("invalid credentials")]
    InvalidCredentials,

    #[error("account locked")]
    AccountLocked,

    #[error("account linking required")]
    RequiresLinking { url: String },

    #[error("not found: {0}")]
    NotFound(String),

    #[error("I/O error: {0}")]
    Io(String),

    #[error("{feature} is not configured")]
    NotConfigured { feature: String },

    #[error("{feature} is not supported on {platform}")]
    UnsupportedPlatform { feature: String, platform: String },

    #[error("{operation} is already in progress")]
    Busy { operation: String },

    #[error("{operation} was cancelled")]
    Cancelled { operation: String },

    #[error("{operation} timed out")]
    Timeout { operation: String },

    #[error("internal error: {0}")]
    Internal(String),

    #[error("webview error: {0}")]
    Webview(String),

    #[error("invalid response: {0}")]
    InvalidResponse(String),

    #[error("invalid input: {0}")]
    InvalidInput(String),
}

pub type CommandResult<T> = Result<T, CommandError>;

impl From<std::io::Error> for CommandError {
    fn from(e: std::io::Error) -> Self {
        Self::Io(e.to_string())
    }
}

impl From<reqwest::Error> for CommandError {
    fn from(e: reqwest::Error) -> Self {
        Self::Network(e.to_string())
    }
}
