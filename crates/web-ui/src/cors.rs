//! CORS policy for the web server.
//!
//! The server is same-origin by default: no CORS headers are emitted unless
//! the operator lists the allowed origins explicitly.

use anyhow::{Result, bail};
use axum::http::{HeaderName, HeaderValue, Method, Uri, header};
use tower_http::cors::CorsLayer;

/// Build the CORS layer for the configured origins.
///
/// Returns `None` when no origins are configured (same-origin only) and an
/// error for any entry that is not a plain `http(s)://host[:port]` origin.
pub(crate) fn build_layer(origins: &[String]) -> Result<Option<CorsLayer>> {
    if origins.is_empty() {
        return Ok(None);
    }
    let allowed = origins
        .iter()
        .map(|origin| parse_origin(origin))
        .collect::<Result<Vec<_>>>()?;
    Ok(Some(
        CorsLayer::new()
            .allow_origin(allowed)
            .allow_methods([
                Method::GET,
                Method::POST,
                Method::PUT,
                Method::PATCH,
                Method::DELETE,
                Method::OPTIONS,
            ])
            .allow_headers([
                header::AUTHORIZATION,
                header::CONTENT_TYPE,
                header::ACCEPT,
                HeaderName::from_static(crate::space::SPACE_HEADER),
            ]),
    ))
}

fn parse_origin(origin: &str) -> Result<HeaderValue> {
    let uri: Uri = origin
        .parse()
        .map_err(|e| anyhow::anyhow!("invalid CORS origin {origin:?}: {e}"))?;
    let scheme_ok = matches!(uri.scheme_str(), Some("http" | "https"));
    let bare = uri.authority().is_some()
        && uri.query().is_none()
        && matches!(uri.path(), "" | "/")
        && !origin.ends_with('/');
    if !scheme_ok || !bare {
        bail!("invalid CORS origin {origin:?}: expected http(s)://host[:port]");
    }
    Ok(HeaderValue::from_str(origin)?)
}

#[cfg(test)]
mod tests {
    use axum::body::Body;
    use axum::http::{Method, Request, header};
    use axum::{Router, routing::get};
    use tower::ServiceExt;

    use super::*;

    fn origins(list: &[&str]) -> Vec<String> {
        list.iter().map(|s| (*s).to_string()).collect()
    }

    fn preflight(origin: &str) -> Request<Body> {
        Request::builder()
            .method(Method::OPTIONS)
            .uri("/api/ping")
            .header(header::ORIGIN, origin)
            .header(header::ACCESS_CONTROL_REQUEST_METHOD, "POST")
            .body(Body::empty())
            .unwrap()
    }

    async fn allow_origin_header(layer: CorsLayer, origin: &str) -> Option<String> {
        let app = Router::new()
            .route("/api/ping", get(|| async { "ok" }))
            .layer(layer);
        let resp = app.oneshot(preflight(origin)).await.unwrap();
        resp.headers()
            .get(header::ACCESS_CONTROL_ALLOW_ORIGIN)
            .map(|v| v.to_str().unwrap().to_string())
    }

    #[test]
    fn no_origins_means_no_cors_layer() {
        assert!(build_layer(&[]).unwrap().is_none());
    }

    #[test]
    fn invalid_origins_are_rejected() {
        for bad in [
            "*",
            "not a url",
            "localhost:3000",
            "ftp://example.com",
            "http://example.com/path",
            "http://example.com/?q=1",
            "",
        ] {
            assert!(
                build_layer(&origins(&[bad])).is_err(),
                "expected {bad:?} to be rejected"
            );
        }
    }

    #[test]
    fn one_invalid_origin_fails_the_whole_list() {
        assert!(build_layer(&origins(&["http://localhost:3000", "*"])).is_err());
    }

    #[tokio::test]
    async fn configured_origin_is_allowed() {
        let layer = build_layer(&origins(&["http://localhost:3000"]))
            .unwrap()
            .unwrap();
        assert_eq!(
            allow_origin_header(layer, "http://localhost:3000")
                .await
                .as_deref(),
            Some("http://localhost:3000")
        );
    }

    #[tokio::test]
    async fn unlisted_origin_gets_no_allow_origin_header() {
        let layer = build_layer(&origins(&["http://localhost:3000"]))
            .unwrap()
            .unwrap();
        assert_eq!(
            allow_origin_header(layer, "http://evil.example").await,
            None
        );
    }

    #[tokio::test]
    async fn multiple_origins_are_each_allowed() {
        for allowed in ["http://localhost:3000", "https://app.example.com"] {
            let layer = build_layer(&origins(&[
                "http://localhost:3000",
                "https://app.example.com",
            ]))
            .unwrap()
            .unwrap();
            assert_eq!(
                allow_origin_header(layer, allowed).await.as_deref(),
                Some(allowed)
            );
        }
    }

    #[tokio::test]
    async fn methods_are_an_explicit_list_not_a_wildcard() {
        let layer = build_layer(&origins(&["http://localhost:3000"]))
            .unwrap()
            .unwrap();
        let app = Router::new()
            .route("/api/ping", get(|| async { "ok" }))
            .layer(layer);
        let resp = app
            .oneshot(preflight("http://localhost:3000"))
            .await
            .unwrap();
        let methods = resp
            .headers()
            .get(header::ACCESS_CONTROL_ALLOW_METHODS)
            .unwrap()
            .to_str()
            .unwrap();
        assert_ne!(methods, "*");
        assert!(methods.contains("POST"));
    }

    #[tokio::test]
    async fn space_selector_header_is_allowed() {
        let layer = build_layer(&origins(&["http://localhost:3000"]))
            .unwrap()
            .unwrap();
        let app = Router::new()
            .route("/api/ping", get(|| async { "ok" }))
            .layer(layer);
        let mut request = preflight("http://localhost:3000");
        request.headers_mut().insert(
            header::ACCESS_CONTROL_REQUEST_HEADERS,
            HeaderValue::from_static("x-space-id"),
        );
        let resp = app.oneshot(request).await.unwrap();
        let allowed = resp
            .headers()
            .get(header::ACCESS_CONTROL_ALLOW_HEADERS)
            .expect("allow-headers")
            .to_str()
            .unwrap()
            .to_ascii_lowercase();
        assert!(allowed.contains("x-space-id"), "{allowed}");
    }
}
