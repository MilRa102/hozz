use std::sync::Arc;

use reqwest::{Method, header::HeaderMap};

use crate::{
    auth::AuthProvider, builder::RestClientBuilder, request::RequestBuilder,
    retry::RetryPolicy,
};

#[derive(Debug)]
pub(crate) struct RestClientState {
    pub(crate) inner: reqwest::Client,
    pub(crate) base_url: Option<String>,
    pub(crate) auth: AuthProvider,
    pub(crate) retry_policy: RetryPolicy,
    pub(crate) default_headers: HeaderMap,
}

/// Thread-safe abstract REST client wrapping `reqwest::Client`.
#[derive(Debug, Clone)]
pub struct RestClient {
    pub(crate) state: Arc<RestClientState>,
}

impl RestClient {
    /// Creates a default `RestClient` with an optional base URL.
    pub fn new(base_url: impl Into<Option<String>>) -> Self {
        let mut builder = RestClientBuilder::new();
        if let Some(url) = base_url.into() {
            builder = builder.base_url(url);
        }
        match builder.build() {
            Ok(client) => client,
            Err(e) => panic!("Failed to build RestClient: {e}"),
        }
    }

    /// Returns a new `RestClientBuilder`.
    pub fn builder() -> RestClientBuilder {
        RestClientBuilder::new()
    }

    /// Returns reference to configured `AuthProvider`.
    pub fn auth(&self) -> &AuthProvider {
        &self.state.auth
    }

    /// Returns reference to configured `RetryPolicy`.
    pub fn retry_policy(&self) -> &RetryPolicy {
        &self.state.retry_policy
    }

    /// Returns optional base URL string.
    pub fn base_url(&self) -> Option<&str> {
        self.state.base_url.as_deref()
    }

    /// Creates a generic request builder for the given HTTP method and path or full URL.
    pub fn request(
        &self,
        method: Method,
        path_or_url: impl Into<String>,
    ) -> RequestBuilder {
        RequestBuilder::new(self.clone(), method, path_or_url)
    }

    /// Shortcut for GET request.
    pub fn get(&self, path_or_url: impl Into<String>) -> RequestBuilder {
        self.request(Method::GET, path_or_url)
    }

    /// Shortcut for POST request.
    pub fn post(&self, path_or_url: impl Into<String>) -> RequestBuilder {
        self.request(Method::POST, path_or_url)
    }

    /// Shortcut for PUT request.
    pub fn put(&self, path_or_url: impl Into<String>) -> RequestBuilder {
        self.request(Method::PUT, path_or_url)
    }

    /// Shortcut for DELETE request.
    pub fn delete(&self, path_or_url: impl Into<String>) -> RequestBuilder {
        self.request(Method::DELETE, path_or_url)
    }

    /// Shortcut for PATCH request.
    pub fn patch(&self, path_or_url: impl Into<String>) -> RequestBuilder {
        self.request(Method::PATCH, path_or_url)
    }
}
