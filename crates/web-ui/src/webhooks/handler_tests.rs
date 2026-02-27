//! Integration tests for webhook HTTP handlers.
//!
//! Uses `tower::ServiceExt::oneshot` with an in-memory SQLite database to test
//! the full request-response cycle without starting a real server.

#[cfg(test)]
mod tests {
    use axum::body::Body;
    use axum::http::{Request, StatusCode};
    use http_body_util::BodyExt;
    use tower::ServiceExt;

    use assistant_storage::{StorageLayer, WebhookStore};

    use crate::webhooks::pages::WebhookPagesState;
    use crate::webhooks::webhook_pages_router;

    /// Build a test router backed by an in-memory database.
    async fn test_app() -> (axum::Router, WebhookPagesState) {
        let storage = StorageLayer::new_in_memory().await.unwrap();
        let state = WebhookPagesState {
            pool: storage.pool.clone(),
        };
        let app = webhook_pages_router().with_state(state.clone());
        (app, state)
    }

    /// Helper: read the full response body as a String.
    async fn body_string(body: Body) -> String {
        let bytes = body.collect().await.unwrap().to_bytes();
        String::from_utf8(bytes.to_vec()).unwrap()
    }

    // -- GET /webhooks (list) --

    #[tokio::test]
    async fn list_webhooks_empty_shows_empty_message() {
        let (app, _state) = test_app().await;
        let resp = app
            .oneshot(
                Request::builder()
                    .uri("/webhooks")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(resp.status(), StatusCode::OK);
        let html = body_string(resp.into_body()).await;
        assert!(html.contains("No webhooks configured yet"));
        assert!(html.contains("<title>Webhooks"));
    }

    #[tokio::test]
    async fn list_webhooks_shows_created_webhook() {
        let (app, state) = test_app().await;

        // Seed a webhook directly via the store.
        let store = WebhookStore::new(state.pool.clone());
        store
            .create("wh-1", "Test Hook", "https://example.com/wh", "secret", &[])
            .await
            .unwrap();

        let resp = app
            .oneshot(
                Request::builder()
                    .uri("/webhooks")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(resp.status(), StatusCode::OK);
        let html = body_string(resp.into_body()).await;
        assert!(html.contains("Test Hook"), "should contain webhook name");
        assert!(
            html.contains("example.com/wh"),
            "should contain webhook URL"
        );
        assert!(!html.contains("No webhooks configured"));
    }

    // -- GET /webhooks/new (create form) --

    #[tokio::test]
    async fn new_webhook_form_renders() {
        let (app, _state) = test_app().await;
        let resp = app
            .oneshot(
                Request::builder()
                    .uri("/webhooks/new")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(resp.status(), StatusCode::OK);
        let html = body_string(resp.into_body()).await;
        assert!(html.contains("Create Webhook"));
        assert!(html.contains("action=\"/webhooks\""));
        assert!(html.contains("turn.result"), "should list event types");
    }

    // -- POST /webhooks (create) --

    #[tokio::test]
    async fn create_webhook_redirects_to_detail() {
        let (app, _state) = test_app().await;
        let resp = app
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/webhooks")
                    .header("Content-Type", "application/x-www-form-urlencoded")
                    .body(Body::from("name=New+Hook&url=https%3A%2F%2Fhook.test"))
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(
            resp.status(),
            StatusCode::SEE_OTHER,
            "should redirect after create",
        );
        let location = resp.headers().get("location").unwrap().to_str().unwrap();
        assert!(
            location.starts_with("/webhooks/"),
            "should redirect to /webhooks/{{id}}, got: {location}",
        );
    }

    #[tokio::test]
    async fn create_webhook_with_events_persists_all() {
        let (app, state) = test_app().await;
        // Event types are comma-separated in the form field.
        let _resp = app
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/webhooks")
                    .header("Content-Type", "application/x-www-form-urlencoded")
                    .body(Body::from(
                        "name=Evented&url=https%3A%2F%2Fe.test\
                         &event_types=turn.result%2C+tool.result",
                    ))
                    .unwrap(),
            )
            .await
            .unwrap();

        let store = WebhookStore::new(state.pool);
        let all = store.list().await.unwrap();
        assert_eq!(all.len(), 1);
        assert_eq!(
            all[0].event_types,
            vec!["turn.result".to_string(), "tool.result".to_string()],
        );
    }

    #[tokio::test]
    async fn create_webhook_persists_in_db() {
        let (app, state) = test_app().await;
        let _resp = app
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/webhooks")
                    .header("Content-Type", "application/x-www-form-urlencoded")
                    .body(Body::from("name=Persisted&url=https%3A%2F%2Fp.test"))
                    .unwrap(),
            )
            .await
            .unwrap();

        let store = WebhookStore::new(state.pool);
        let all = store.list().await.unwrap();
        assert_eq!(all.len(), 1);
        assert_eq!(all[0].name, "Persisted");
        assert_eq!(all[0].url, "https://p.test");
        assert!(all[0].active, "should be active by default");
        assert_eq!(all[0].secret.len(), 64, "should have a 64-char hex secret");
    }

    // -- GET /webhooks/:id (detail) --

    #[tokio::test]
    async fn show_webhook_renders_detail() {
        let (app, state) = test_app().await;
        let store = WebhookStore::new(state.pool.clone());
        store
            .create(
                "wh-detail",
                "Detail Hook",
                "https://detail.test",
                "mysecret",
                &["turn.result".to_string()],
            )
            .await
            .unwrap();

        let resp = app
            .oneshot(
                Request::builder()
                    .uri("/webhooks/wh-detail")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(resp.status(), StatusCode::OK);
        let html = body_string(resp.into_body()).await;
        assert!(html.contains("Detail Hook"));
        assert!(html.contains("detail.test"));
        assert!(
            html.contains("mysecret"),
            "secret should be in the page (masked by CSS)"
        );
        assert!(html.contains("turn.result"));
        assert!(
            html.contains("unverified"),
            "new webhook should be unverified"
        );
    }

    #[tokio::test]
    async fn show_webhook_not_found_returns_404() {
        let (app, _state) = test_app().await;
        let resp = app
            .oneshot(
                Request::builder()
                    .uri("/webhooks/nonexistent")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(resp.status(), StatusCode::NOT_FOUND);
    }

    // -- POST /webhooks/:id/delete --

    #[tokio::test]
    async fn delete_webhook_removes_and_redirects() {
        let (app, state) = test_app().await;
        let store = WebhookStore::new(state.pool.clone());
        store
            .create("wh-del", "Delete Me", "https://del.test", "s", &[])
            .await
            .unwrap();

        let resp = app
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/webhooks/wh-del/delete")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(resp.status(), StatusCode::SEE_OTHER);
        let location = resp.headers().get("location").unwrap().to_str().unwrap();
        assert_eq!(location, "/webhooks");

        // Confirm it's gone.
        assert!(store.get("wh-del").await.unwrap().is_none());
    }

    // -- POST /webhooks/:id/toggle --

    #[tokio::test]
    async fn toggle_webhook_flips_active() {
        let (_app, state) = test_app().await;
        let store = WebhookStore::new(state.pool.clone());
        store
            .create("wh-tog", "Toggle", "https://tog.test", "s", &[])
            .await
            .unwrap();
        assert!(store.get("wh-tog").await.unwrap().unwrap().active);

        // Need to rebuild the router for the second request since oneshot consumes it.
        let app1 = webhook_pages_router().with_state(WebhookPagesState {
            pool: state.pool.clone(),
        });
        let resp = app1
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/webhooks/wh-tog/toggle")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(resp.status(), StatusCode::SEE_OTHER);
        assert!(!store.get("wh-tog").await.unwrap().unwrap().active);
    }

    // -- POST /webhooks/:id/rotate-secret --

    #[tokio::test]
    async fn rotate_secret_changes_secret_and_clears_verified() {
        let (app, state) = test_app().await;
        let store = WebhookStore::new(state.pool.clone());
        store
            .create("wh-rot", "Rotate", "https://rot.test", "original", &[])
            .await
            .unwrap();
        store.mark_verified("wh-rot").await.unwrap();

        let resp = app
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/webhooks/wh-rot/rotate-secret")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(resp.status(), StatusCode::SEE_OTHER);
        let wh = store.get("wh-rot").await.unwrap().unwrap();
        assert_ne!(wh.secret, "original", "secret should have changed");
        assert_eq!(wh.secret.len(), 64, "new secret should be 64 hex chars");
        assert!(wh.verified_at.is_none(), "verification should be cleared");
    }

    // -- POST /webhooks/:id/edit --

    #[tokio::test]
    async fn update_webhook_changes_fields() {
        let (app, state) = test_app().await;
        let store = WebhookStore::new(state.pool.clone());
        store
            .create(
                "wh-upd",
                "Old Name",
                "https://old.test",
                "s",
                &["turn.result".to_string()],
            )
            .await
            .unwrap();

        let resp = app
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/webhooks/wh-upd/edit")
                    .header("Content-Type", "application/x-www-form-urlencoded")
                    .body(Body::from(
                        "name=New+Name&url=https%3A%2F%2Fnew.test&active=on\
                         &event_types=tool.result%2C+agent.spawn",
                    ))
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(resp.status(), StatusCode::SEE_OTHER);
        let wh = store.get("wh-upd").await.unwrap().unwrap();
        assert_eq!(wh.name, "New Name");
        assert_eq!(wh.url, "https://new.test");
        assert!(wh.active);
        assert_eq!(
            wh.event_types,
            vec!["tool.result".to_string(), "agent.spawn".to_string()],
        );
    }

    // -- GET /webhooks/:id/edit --

    #[tokio::test]
    async fn edit_form_prepopulates_fields() {
        let (app, state) = test_app().await;
        let store = WebhookStore::new(state.pool.clone());
        store
            .create(
                "wh-edit",
                "Edit Me",
                "https://edit.test",
                "s",
                &["agent.spawn".to_string()],
            )
            .await
            .unwrap();

        let resp = app
            .oneshot(
                Request::builder()
                    .uri("/webhooks/wh-edit/edit")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(resp.status(), StatusCode::OK);
        let html = body_string(resp.into_body()).await;
        assert!(html.contains("Edit Me"));
        assert!(html.contains("edit.test"));
        assert!(html.contains("Save Changes"));
    }

    #[tokio::test]
    async fn edit_form_not_found_returns_404() {
        let (app, _state) = test_app().await;
        let resp = app
            .oneshot(
                Request::builder()
                    .uri("/webhooks/nonexistent/edit")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(resp.status(), StatusCode::NOT_FOUND);
    }
}
