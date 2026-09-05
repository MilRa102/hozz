use std::sync::Arc;

use async_trait::async_trait;
use reqwest::header::{AUTHORIZATION, HeaderMap, HeaderName, HeaderValue};

use crate::error::RestError;

/// Trait for dynamically providing authentication tokens (e.g. OAuth tokens, refreshed JWTs).
#[async_trait]
pub trait DynamicTokenProvider: Send + Sync {
    /// Retrieves a valid token string (e.g., Bearer token).
    async fn get_token(&self) -> Result<String, RestError>;
}

/// Authentication mechanism for REST API requests.
#[derive(Clone, Default)]
pub enum AuthProvider {
    /// No authentication.
    #[default]
    None,
    /// Static Bearer token.
    Bearer(String),
    /// Single static header key-value pair.
    Header(HeaderName, HeaderValue),
    /// Multiple static headers.
    CustomHeaders(HeaderMap),
    /// Dynamic token provider for refreshed tokens.
    Dynamic(Arc<dyn DynamicTokenProvider>),
}

impl AuthProvider {
    /// Creates an `AuthProvider::Bearer` with the given token.
    pub fn bearer(token: impl Into<String>) -> Self {
        Self::Bearer(token.into())
    }

    /// Creates an `AuthProvider::Header` with the given name and value.
    pub fn header(name: HeaderName, value: HeaderValue) -> Self {
        Self::Header(name, value)
    }

    /// Creates an `AuthProvider::Dynamic` with a custom token provider.
    pub fn dynamic<P: DynamicTokenProvider + 'static>(provider: P) -> Self {
        Self::Dynamic(Arc::new(provider))
    }

    /// Appends authorization headers to the given `HeaderMap`.
    pub async fn apply_headers(&self, headers: &mut HeaderMap) -> Result<(), RestError> {
        match self {
            Self::None => {},
            Self::Bearer(token) => {
                let auth_val = format!("Bearer {token}");
                let header_val = HeaderValue::from_str(&auth_val).map_err(|e| {
                    RestError::Auth(format!("Invalid bearer header value: {e}"))
                })?;
                headers.insert(AUTHORIZATION, header_val);
            },
            Self::Header(name, value) => {
                headers.insert(name.clone(), value.clone());
            },
            Self::CustomHeaders(custom) => {
                for (k, v) in custom {
                    headers.insert(k.clone(), v.clone());
                }
            },
            Self::Dynamic(provider) => {
                let token = provider.get_token().await?;
                let auth_val = if token.starts_with("Bearer ") {
                    token
                } else {
                    format!("Bearer {token}")
                };
                let header_val = HeaderValue::from_str(&auth_val).map_err(|e| {
                    RestError::Auth(format!("Invalid dynamic token value: {e}"))
                })?;
                headers.insert(AUTHORIZATION, header_val);
            },
        }
        Ok(())
    }
}

impl std::fmt::Debug for AuthProvider {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::None => write!(f, "AuthProvider::None"),
            Self::Bearer(_) => write!(f, "AuthProvider::Bearer(***)"),
            Self::Header(name, _) => write!(f, "AuthProvider::Header({name}, ***)"),
            Self::CustomHeaders(headers) => write!(
                f,
                "AuthProvider::CustomHeaders(keys: {:?})",
                headers.keys().collect::<Vec<_>>()
            ),
            Self::Dynamic(_) => write!(
                f,
                "AuthProvider::Dynamic(<dyn DynamicTokenProvider>)"
            ),
        }
    }
}
