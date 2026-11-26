//! Media generated with a final answer is stored as attachments of the
//! assistant message.

use std::sync::Arc;

use assistant_core::types::conversation::{Interface, TurnIdentity};
use assistant_core::{GeneratedMedia, LlmResponse, LlmResponseMeta, MediaKind};
use assistant_llm_provider::scripted::ScriptedLlmProvider;
use assistant_storage::AttachmentStore as _;
use uuid::Uuid;

use super::*;

#[tokio::test]
async fn final_answer_media_becomes_assistant_attachments() {
    let png = vec![0x89, b'P', b'N', b'G'];
    let llm = ScriptedLlmProvider::new().with_canned_responses(vec![LlmResponse::FinalAnswer(
        "Here is your picture.".into(),
        LlmResponseMeta {
            media: vec![GeneratedMedia {
                kind: MediaKind::Image,
                mime: "image/png".into(),
                data: bytes::Bytes::from(png.clone()),
            }],
            ..LlmResponseMeta::default()
        },
    )]);
    let mut config = OrgConfig::default();
    config.memory.enabled = false;
    let (orch, storage) = build_with_llm(Arc::new(llm), config, |o| o).await;

    let result = orch
        .run_turn_with_tools(
            "draw a cat",
            Uuid::new_v4(),
            Interface::Web,
            vec![],
            None,
            vec![],
            vec![],
            TurnIdentity::default(),
        )
        .await
        .unwrap();

    assert_eq!(
        result.attachment_ids.len(),
        1,
        "{:?}",
        result.attachment_ids
    );
    let store = storage.attachment_store();
    let meta = store.get_meta(result.attachment_ids[0]).await.unwrap();
    assert_eq!(meta.mime_type, "image/png");
    assert_eq!(
        meta.message_id, result.message_id,
        "linked to the assistant message"
    );
    assert_eq!(store.load_bytes(meta.id).await.unwrap(), png);
    assert_eq!(result.attachments.len(), 1);
    assert_eq!(result.attachments[0].mime_type, "image/png");
    assert!(
        result.attachments[0].filename.ends_with(".png"),
        "{}",
        result.attachments[0].filename
    );
}
