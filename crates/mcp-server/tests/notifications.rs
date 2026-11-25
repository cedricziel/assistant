//! List-changed notifications after `install-skill`, under both protocol
//! revisions: `subscriptions/listen` streams for `2026-07-28` and classic
//! notifications for `2025-11-25` sessions.

mod common;

use std::collections::HashSet;
use std::time::Duration;

use common::*;
use rmcp::ServiceExt;
use rmcp::model::{GetMeta, ProtocolVersion, ServerNotification, SubscriptionFilter};
use rmcp::service::{ClientLifecycleMode, serve_client_with_lifecycle};

#[tokio::test]
async fn initialize_advertises_list_changed_for_tools_resources_and_prompts() {
    let client = default_stubs().connect().await;
    let capabilities = &client.peer_info().expect("peer info").capabilities;

    let list_changed =
        |name: &str| serde_json::to_value(capabilities).unwrap()[name]["listChanged"] == true;
    assert!(list_changed("tools"), "{capabilities:?}");
    assert!(list_changed("resources"), "{capabilities:?}");
    assert!(list_changed("prompts"), "{capabilities:?}");
}

#[tokio::test]
async fn listening_session_is_notified_when_another_session_installs_a_skill() {
    let server = installing_server().await;
    let modern = ClientLifecycleMode::Discover {
        preferred_versions: vec![ProtocolVersion::V_2026_07_28],
    };
    let listener = serve_client_with_lifecycle((), spawn_server(server.clone()), modern)
        .await
        .expect("discover handshake");
    let installer = connect(server).await;
    let mut subscription = listener
        .listen(
            SubscriptionFilter::builder()
                .tools_list_changed()
                .prompts_list_changed()
                .build(),
        )
        .await
        .expect("listen is accepted");

    let (_source, params) = install_params("fresh-skill");
    installer.call_tool(params).await.unwrap();

    let mut kinds = HashSet::new();
    for _ in 0..2 {
        let notification = tokio::time::timeout(Duration::from_secs(5), subscription.next())
            .await
            .expect("a notification within five seconds")
            .expect("a well-formed stream")
            .expect("the subscription is open");
        assert_eq!(
            notification.get_meta().subscription_id().as_ref(),
            Some(subscription.id())
        );
        kinds.insert(match notification {
            ServerNotification::ToolListChangedNotification(_) => "tools",
            ServerNotification::PromptListChangedNotification(_) => "prompts",
            other => panic!("unexpected notification {other:?}"),
        });
    }
    assert_eq!(kinds, HashSet::from(["tools", "prompts"]));
}

#[tokio::test]
async fn legacy_session_receives_classic_notifications_when_a_skill_is_installed() {
    let server = installing_server().await;
    let (recorder, mut received) = Recorder::new(ProtocolVersion::V_2025_11_25);
    let _legacy = recorder
        .serve(spawn_server(server.clone()))
        .await
        .expect("handshake");
    let installer = connect(server).await;

    let (_source, params) = install_params("fresh-skill");
    installer.call_tool(params).await.unwrap();

    let mut kinds = Vec::new();
    for _ in 0..3 {
        kinds.push(next_received(&mut received).await);
    }
    for expected in [
        Received::ToolsChanged,
        Received::PromptsChanged,
        Received::ResourcesChanged,
    ] {
        assert!(kinds.contains(&expected), "{expected:?} in {kinds:?}");
    }
}
