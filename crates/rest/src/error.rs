use thiserror::Error;

/// Errors that can occur when using the REST API client.
#[derive(Debug, Error)]
pub enum RestError {
    /// Network or HTTP transport error from `reqwest`.
    #[error("HTTP transport error: {0}")]
    Http(#[from] reqwest::Error),

    /// API error response with HTTP status code and body.
    #[error("API error response (status {status}): {body}")]
    Api {
        status: reqwest::StatusCode,
        body: String,
    },

    /// JSON serialization or deserialization error.
    #[error("JSON error: {0}")]
    Json(#[from] serde_json::Error),

    /// Authentication error (e.g. failed to retrieve dynamic token).
    #[error("Authentication error: {0}")]
    Auth(String),

    /// Maximum retry attempts exceeded.
    #[error("Max retries ({attempts}) exceeded. Last error: {last_error}")]
    MaxRetriesExceeded {
        attempts: u32,
        last_error: Box<RestError>,
    },

    /// Invalid URL construction.
    #[error("Invalid URL: {0}")]
    InvalidUrl(String),

    /// Invalid request or client build configuration.
    #[error("Builder error: {0}")]
    Builder(String),
}

impl RestError {
    /// Returns the HTTP status code if this error is an `Api` error or contains a `reqwest::Error` status.
    pub fn status_code(&self) -> Option<reqwest::StatusCode> {
        match self {
            Self::Api { status, .. } => Some(*status),
            Self::Http(err) => err.status(),
            Self::MaxRetriesExceeded { last_error, .. } => last_error.status_code(),
            _ => None,
        }
    }

    /// Returns `true` if the error represents a transient issue suitable for retry.
    pub fn is_transient(&self) -> bool {
        match self {
            Self::Http(err) => err.is_timeout() || err.is_connect() || err.is_request(),
            Self::Api { status, .. } => {
                status.is_server_error()
                    || *status == reqwest::StatusCode::TOO_MANY_REQUESTS
            },
            Self::MaxRetriesExceeded { last_error, .. } => last_error.is_transient(),
            _ => false,
        }
    }
}
