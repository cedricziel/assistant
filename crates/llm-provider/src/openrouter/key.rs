//! Key health from OpenRouter's `GET /key`.

use serde::Deserialize;

use assistant_core::Secret;

use crate::retry::RetryConfig;

/// What `GET /key` said about a key.
#[derive(Debug, Clone, PartialEq)]
pub enum KeyStatus {
    /// The key works. `limit_remaining` is `None` when the key has no limit.
    Valid {
        limit_remaining: Option<f64>,
        free_tier: bool,
    },
    /// OpenRouter answered 401.
    Invalid,
    /// Any other failure, with the reason.
    Unreachable(String),
}

#[derive(Debug, Deserialize)]
struct KeyResponse {
    data: KeyData,
}

#[derive(Debug, Deserialize)]
struct KeyData {
    #[serde(default)]
    limit_remaining: Option<f64>,
    #[serde(default)]
    is_free_tier: bool,
}

/// Asks `{base_url}/key` about `key`.
pub async fn key_status(base_url: &str, key: &Secret) -> KeyStatus {
    let http = match crate::http::build_http_client(10, &RetryConfig::disabled()) {
        Ok(http) => http,
        Err(e) => return KeyStatus::Unreachable(e.to_string()),
    };
    let url = format!("{}/key", base_url.trim_end_matches('/'));
    let resp = match http.get(&url).bearer_auth(key.expose()).send().await {
        Ok(resp) => resp,
        Err(e) => return KeyStatus::Unreachable(e.to_string()),
    };
    match resp.status().as_u16() {
        401 => KeyStatus::Invalid,
        200..=299 => match resp.json::<KeyResponse>().await {
            Ok(body) => KeyStatus::Valid {
                limit_remaining: body.data.limit_remaining,
                free_tier: body.data.is_free_tier,
            },
            Err(e) => KeyStatus::Unreachable(format!("unreadable /key response: {e}")),
        },
        status => KeyStatus::Unreachable(format!("/key returned {status}")),
    }
}

#[cfg(test)]
mod tests {
    use serde_json::json;
    use wiremock::matchers::{header, method, path};
    use wiremock::{Mock, MockServer, ResponseTemplate};

    use super::*;

    async fn status_for(response: ResponseTemplate) -> KeyStatus {
        let server = MockServer::start().await;
        Mock::given(method("GET"))
            .and(path("/key"))
            .and(header("authorization", "Bearer sk-or-test"))
            .respond_with(response)
            .expect(1)
            .mount(&server)
            .await;
        key_status(&server.uri(), &Secret::new("sk-or-test")).await
    }

    #[tokio::test]
    async fn valid_key_reports_its_limit_and_tier() {
        let body = json!({"data": {"label": "x", "limit": 20.0, "limit_remaining": 12.5, "usage": 7.5, "is_free_tier": false}});
        assert_eq!(
            status_for(ResponseTemplate::new(200).set_body_json(body)).await,
            KeyStatus::Valid {
                limit_remaining: Some(12.5),
                free_tier: false
            }
        );
    }

    #[tokio::test]
    async fn unlimited_free_tier_key() {
        let body = json!({"data": {"limit": null, "limit_remaining": null, "is_free_tier": true}});
        assert_eq!(
            status_for(ResponseTemplate::new(200).set_body_json(body)).await,
            KeyStatus::Valid {
                limit_remaining: None,
                free_tier: true
            }
        );
    }

    #[tokio::test]
    async fn unauthorized_is_invalid_and_other_failures_unreachable() {
        assert_eq!(
            status_for(ResponseTemplate::new(401)).await,
            KeyStatus::Invalid
        );
        assert!(matches!(
            status_for(ResponseTemplate::new(503)).await,
            KeyStatus::Unreachable(reason) if reason.contains("503")
        ));
        let closed = key_status("http://127.0.0.1:9", &Secret::new("k")).await;
        assert!(matches!(closed, KeyStatus::Unreachable(_)), "{closed:?}");
    }
}
