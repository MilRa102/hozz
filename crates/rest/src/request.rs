use std::time::Duration;

use reqwest::{
    Method, Response,
    header::{HeaderMap, HeaderName, HeaderValue},
};
use serde::{Serialize, de::DeserializeOwned};
use tracing::warn;

use crate::{client::RestClient, error::RestError, retry::RetryPolicy};

/// Builder for forming and sending an individual HTTP REST request.
#[derive(Debug, Clone)]
pub struct RequestBuilder {
    client: RestClient,
    method: Method,
    path_or_url: String,
    query: Vec<(String, String)>,
    headers: HeaderMap,
    body: Option<Vec<u8>>,
    timeout: Option<Duration>,
    retry_policy: Option<RetryPolicy>,
}

impl RequestBuilder {
    pub(crate) fn new(
        client: RestClient,
        method: Method,
        path_or_url: impl Into<String>,
    ) -> Self {
        Self {
            client,
            method,
            path_or_url: path_or_url.into(),
            query: Vec::new(),
            headers: HeaderMap::new(),
            body: None,
            timeout: None,
            retry_policy: None,
        }
    }

    /// Adds a query parameter pair.
    pub fn query(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.query.push((key.into(), value.into()));
        self
    }

    /// Adds multiple query parameters.
    pub fn query_map<I, K, V>(mut self, items: I) -> Self
    where
        I: IntoIterator<Item = (K, V)>,
        K: Into<String>,
        V: Into<String>,
    {
        for (k, v) in items {
            self.query.push((k.into(), v.into()));
        }
        self
    }

    /// Adds a custom header to this request.
    pub fn header(mut self, name: HeaderName, value: HeaderValue) -> Self {
        self.headers.insert(name, value);
        self
    }

    /// Extends headers with a map.
    pub fn headers(mut self, headers: HeaderMap) -> Self {
        self.headers.extend(headers);
        self
    }

    /// Serializes standard payload to JSON and sets `Content-Type: application/json`.
    pub fn json<T: Serialize>(mut self, payload: &T) -> Result<Self, RestError> {
        let bytes = serde_json::to_vec(payload)?;
        self.headers.insert(
            reqwest::header::CONTENT_TYPE,
            HeaderValue::from_static("application/json"),
        );
        self.body = Some(bytes);
        Ok(self)
    }

    /// Sets raw byte payload.
    pub fn body(mut self, body: impl Into<Vec<u8>>) -> Self {
        self.body = Some(body.into());
        self
    }

    /// Overrides per-request timeout.
    pub fn timeout(mut self, timeout: Duration) -> Self {
        self.timeout = Some(timeout);
        self
    }

    /// Overrides per-request retry policy.
    pub fn retry_policy(mut self, policy: RetryPolicy) -> Self {
        self.retry_policy = Some(policy);
        self
    }

    /// Builds the target URL considering `base_url` configuration.
    pub(crate) fn build_url(&self) -> Result<String, RestError> {
        if self.path_or_url.starts_with("http://")
            || self.path_or_url.starts_with("https://")
        {
            Ok(self.path_or_url.clone())
        } else if let Some(base) = self.client.base_url() {
            let path = self.path_or_url.trim_start_matches('/');
            if path.is_empty() {
                Ok(base.to_string())
            } else {
                Ok(format!("{base}/{path}"))
            }
        } else {
            Ok(self.path_or_url.clone())
        }
    }

    /// Sends the HTTP request with automatic retries, auth injection, and error handling.
    pub async fn send(self) -> Result<Response, RestError> {
        let raw_url = self.build_url()?;
        let mut parsed_url = reqwest::Url::parse(&raw_url)
            .map_err(|e| RestError::InvalidUrl(e.to_string()))?;

        if !self.query.is_empty() {
            parsed_url
                .query_pairs_mut()
                .extend_pairs(&self.query);
        }

        let policy = self
            .retry_policy
            .as_ref()
            .unwrap_or(&self.client.state.retry_policy);

        let mut last_error: Option<RestError> = None;

        for attempt in 1..=policy.max_attempts {
            let mut req_headers = self.client.state.default_headers.clone();
            req_headers.extend(self.headers.clone());

            // Apply authentication headers on every attempt (refreshing dynamic tokens if needed)
            self.client
                .state
                .auth
                .apply_headers(&mut req_headers)
                .await?;

            let mut req_builder = self
                .client
                .state
                .inner
                .request(self.method.clone(), parsed_url.clone())
                .headers(req_headers);

            if let Some(ref body_bytes) = self.body {
                req_builder = req_builder.body(body_bytes.clone());
            }

            if let Some(t) = self.timeout {
                req_builder = req_builder.timeout(t);
            }

            match req_builder.send().await {
                Ok(response) => {
                    let status = response.status();
                    if status.is_success() {
                        return Ok(response);
                    }

                    let body_text = response.text().await.unwrap_or_default();
                    let api_err = RestError::Api {
                        status,
                        body: body_text,
                    };

                    if policy.should_retry(attempt, &api_err) {
                        let delay = policy.calculate_delay(attempt);
                        warn!(
                            url = %parsed_url,
                            attempt,
                            max_attempts = policy.max_attempts,
                            status = %status,
                            delay_ms = delay.as_millis(),
                            "REST request failed with transient API error, retrying..."
                        );
                        last_error = Some(api_err);
                        tokio::time::sleep(delay).await;
                        continue;
                    }

                    return Err(api_err);
                },
                Err(reqwest_err) => {
                    let http_err = RestError::Http(reqwest_err);

                    if policy.should_retry(attempt, &http_err) {
                        let delay = policy.calculate_delay(attempt);
                        warn!(
                            url = %parsed_url,
                            attempt,
                            max_attempts = policy.max_attempts,
                            delay_ms = delay.as_millis(),
                            error = %http_err,
                            "REST request failed with transport error, retrying..."
                        );
                        last_error = Some(http_err);
                        tokio::time::sleep(delay).await;
                        continue;
                    }

                    return Err(http_err);
                },
            }
        }

        Err(RestError::MaxRetriesExceeded {
            attempts: policy.max_attempts,
            last_error: Box::new(last_error.unwrap_or_else(|| {
                RestError::Builder("Unknown error during retry loop".into())
            })),
        })
    }

    /// Sends request and deserializes the JSON response body into `T`.
    pub async fn json_response<T: DeserializeOwned>(self) -> Result<T, RestError> {
        let response = self.send().await?;
        let bytes = response.bytes().await?;
        let data = serde_json::from_slice::<T>(&bytes)?;
        Ok(data)
    }

    /// Sends request and reads response body as `String`.
    pub async fn text(self) -> Result<String, RestError> {
        let response = self.send().await?;
        let text = response.text().await?;
        Ok(text)
    }

    /// Sends request and reads response body as `bytes::Bytes`.
    pub async fn bytes(self) -> Result<bytes::Bytes, RestError> {
        let response = self.send().await?;
        let bytes = response.bytes().await?;
        Ok(bytes)
    }
}
