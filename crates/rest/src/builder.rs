use std::{sync::Arc, time::Duration};

use reqwest::header::{HeaderMap, HeaderName, HeaderValue};

use crate::{
    auth::AuthProvider,
    client::{RestClient, RestClientState},
    error::RestError,
    retry::RetryPolicy,
};

/// Builder for constructing a custom [`RestClient`].
#[derive(Debug, Default)]
pub struct RestClientBuilder {
    base_url: Option<String>,
    auth: AuthProvider,
    retry_policy: RetryPolicy,
    default_headers: HeaderMap,
    timeout: Option<Duration>,
    user_agent: Option<String>,
    inner_client: Option<reqwest::Client>,
}

impl RestClientBuilder {
    /// Creates a new `RestClientBuilder` with default settings.
    pub fn new() -> Self {
        Self::default()
    }

    /// Sets the base URL for relative request paths.
    pub fn base_url(mut self, base_url: impl Into<String>) -> Self {
        let url = base_url.into();
        self.base_url = Some(if url.ends_with('/') {
            url.trim_end_matches('/').to_string()
        } else {
            url
        });
        self
    }

    /// Sets the authentication provider.
    pub fn auth(mut self, auth: AuthProvider) -> Self {
        self.auth = auth;
        self
    }

    /// Shortcut to set static Bearer token authentication.
    pub fn bearer_auth(self, token: impl Into<String>) -> Self {
        self.auth(AuthProvider::bearer(token))
    }

    /// Sets retry policy.
    pub fn retry_policy(mut self, policy: RetryPolicy) -> Self {
        self.retry_policy = policy;
        self
    }

    /// Sets a default HTTP header for all requests.
    pub fn default_header(mut self, name: HeaderName, value: HeaderValue) -> Self {
        self.default_headers.insert(name, value);
        self
    }

    /// Sets default headers.
    pub fn default_headers(mut self, headers: HeaderMap) -> Self {
        self.default_headers.extend(headers);
        self
    }

    /// Sets default request timeout on the underlying `reqwest::Client`.
    pub fn timeout(mut self, timeout: Duration) -> Self {
        self.timeout = Some(timeout);
        self
    }

    /// Sets custom User-Agent header.
    pub fn user_agent(mut self, user_agent: impl Into<String>) -> Self {
        self.user_agent = Some(user_agent.into());
        self
    }

    /// Uses an existing `reqwest::Client` instance instead of constructing a new one.
    pub fn reqwest_client(mut self, client: reqwest::Client) -> Self {
        self.inner_client = Some(client);
        self
    }

    /// Builds and returns the configured [`RestClient`].
    pub fn build(self) -> Result<RestClient, RestError> {
        let inner = match self.inner_client {
            Some(c) => c,
            None => {
                let mut reqwest_builder = reqwest::Client::builder();
                if let Some(t) = self.timeout {
                    reqwest_builder = reqwest_builder.timeout(t);
                }
                if let Some(ua) = self.user_agent {
                    reqwest_builder = reqwest_builder.user_agent(ua);
                }
                reqwest_builder
                    .build()
                    .map_err(|e| RestError::Builder(e.to_string()))?
            },
        };

        Ok(RestClient {
            state: Arc::new(RestClientState {
                inner,
                base_url: self.base_url,
                auth: self.auth,
                retry_policy: self.retry_policy,
                default_headers: self.default_headers,
            }),
        })
    }
}
