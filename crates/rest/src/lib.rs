pub mod auth;
pub mod builder;
pub mod client;
pub mod error;
pub mod request;
pub mod retry;

pub use auth::{AuthProvider, DynamicTokenProvider};
pub use builder::RestClientBuilder;
pub use client::RestClient;
pub use error::RestError;
pub use request::RequestBuilder;
pub use retry::RetryPolicy;

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used)]
mod tests {
    use std::{
        sync::atomic::{AtomicU32, Ordering},
        time::Duration,
    };

    use async_trait::async_trait;
    use reqwest::header::{AUTHORIZATION, HeaderMap, HeaderValue};

    use super::*;

    struct MockTokenProvider {
        counter: AtomicU32,
    }

    #[async_trait]
    impl DynamicTokenProvider for MockTokenProvider {
        async fn get_token(&self) -> Result<String, RestError> {
            let count = self.counter.fetch_add(1, Ordering::SeqCst);
            Ok(format!("token-{count}"))
        }
    }

    #[tokio::test]
    async fn test_auth_provider_bearer() {
        let auth = AuthProvider::bearer("my-secret-token");
        let mut headers = HeaderMap::new();
        auth.apply_headers(&mut headers).await.unwrap();

        assert_eq!(
            headers.get(AUTHORIZATION).unwrap(),
            HeaderValue::from_static("Bearer my-secret-token")
        );
    }

    #[tokio::test]
    async fn test_auth_provider_dynamic() {
        let provider = MockTokenProvider {
            counter: AtomicU32::new(1),
        };
        let auth = AuthProvider::dynamic(provider);

        let mut headers1 = HeaderMap::new();
        auth.apply_headers(&mut headers1).await.unwrap();
        assert_eq!(
            headers1.get(AUTHORIZATION).unwrap(),
            HeaderValue::from_static("Bearer token-1")
        );

        let mut headers2 = HeaderMap::new();
        auth.apply_headers(&mut headers2).await.unwrap();
        assert_eq!(
            headers2.get(AUTHORIZATION).unwrap(),
            HeaderValue::from_static("Bearer token-2")
        );
    }

    #[test]
    fn test_retry_policy_delay_and_backoff() {
        let policy = RetryPolicy::new(5, Duration::from_millis(100))
            .with_backoff_factor(2.0)
            .with_max_delay(Duration::from_secs(1))
            .with_jitter(false);

        assert_eq!(
            policy.calculate_delay(1),
            Duration::from_millis(100)
        );
        assert_eq!(
            policy.calculate_delay(2),
            Duration::from_millis(200)
        );
        assert_eq!(
            policy.calculate_delay(3),
            Duration::from_millis(400)
        );
        assert_eq!(
            policy.calculate_delay(4),
            Duration::from_millis(800)
        );
        assert_eq!(policy.calculate_delay(5), Duration::from_secs(1));
    }

    #[test]
    fn test_retry_policy_should_retry() {
        let policy = RetryPolicy::new(3, Duration::from_millis(100));

        let api_500 = RestError::Api {
            status: reqwest::StatusCode::INTERNAL_SERVER_ERROR,
            body: "Internal Error".into(),
        };

        let api_400 = RestError::Api {
            status: reqwest::StatusCode::BAD_REQUEST,
            body: "Bad Request".into(),
        };

        assert!(policy.should_retry(1, &api_500));
        assert!(policy.should_retry(2, &api_500));
        assert!(!policy.should_retry(3, &api_500)); // Exceeded max_attempts

        assert!(!policy.should_retry(1, &api_400)); // 400 Bad Request is non-transient
    }

    #[test]
    fn test_url_building() {
        let client = RestClient::builder()
            .base_url("https://api.example.com/v1")
            .build()
            .unwrap();

        let req1 = client.get("/users");
        assert_eq!(
            req1.build_url().unwrap(),
            "https://api.example.com/v1/users"
        );

        let req2 = client.get("users");
        assert_eq!(
            req2.build_url().unwrap(),
            "https://api.example.com/v1/users"
        );

        let req3 = client.get("https://custom.domain.com/endpoint");
        assert_eq!(
            req3.build_url().unwrap(),
            "https://custom.domain.com/endpoint"
        );
    }
}
