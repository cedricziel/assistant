//! Contract tests for [`IntegrationCredentialStore`], run against SQLite and in-memory.

use assistant_core::identity::UserId;
use assistant_core::integrations::*;

use super::integration_store::{Fixture, at, memory, org_integration, sqlite};

pub fn credential(integration_id: &str, subject: Option<&str>, updated: i64) -> StoredCredential {
    StoredCredential {
        integration_id: integration_id.into(),
        subject_user_id: subject.map(UserId::from),
        kind: CredentialKind::Header,
        ciphertext: vec![1, 2, 3, 4],
        nonce: vec![9; 12],
        key_id: "abcd1234abcd1234".into(),
        expires_at: None,
        status: CredentialStatus::Ready,
        updated_at: at(updated),
    }
}

async fn seeded(fx: Fixture) -> Fixture {
    fx.store
        .create(&org_integration("int_1", "gh"))
        .await
        .unwrap();
    fx
}

async fn shared_and_per_user_subjects_are_independent(fx: Fixture) {
    let fx = seeded(fx).await;
    let u1 = UserId::from("usr_1");
    let u2 = UserId::from("usr_2");
    let mut shared = credential("int_1", None, 1);
    shared.ciphertext = vec![0xaa];
    let mut mine = credential("int_1", Some("usr_1"), 2);
    mine.ciphertext = vec![0xbb];
    fx.credentials.put(&shared).await.unwrap();
    fx.credentials.put(&mine).await.unwrap();

    assert_eq!(
        fx.credentials.get("int_1", None).await.unwrap(),
        Some(shared)
    );
    assert_eq!(
        fx.credentials.get("int_1", Some(&u1)).await.unwrap(),
        Some(mine)
    );
    assert_eq!(fx.credentials.get("int_1", Some(&u2)).await.unwrap(), None);
    assert_eq!(fx.credentials.get("missing", None).await.unwrap(), None);
}

async fn put_replaces_the_existing_subject_row(fx: Fixture) {
    let fx = seeded(fx).await;
    fx.credentials
        .put(&credential("int_1", None, 1))
        .await
        .unwrap();
    let mut next = credential("int_1", None, 2);
    next.kind = CredentialKind::Oauth;
    next.ciphertext = vec![7, 7];
    next.status = CredentialStatus::AuthExpired;
    next.expires_at = Some(at(3600));
    fx.credentials.put(&next).await.unwrap();
    assert_eq!(fx.credentials.get("int_1", None).await.unwrap(), Some(next));
}

async fn delete_removes_only_that_subject(fx: Fixture) {
    let fx = seeded(fx).await;
    fx.credentials
        .put(&credential("int_1", None, 1))
        .await
        .unwrap();
    fx.credentials
        .put(&credential("int_1", Some("usr_1"), 1))
        .await
        .unwrap();

    assert!(
        fx.credentials
            .delete("int_1", Some(&UserId::from("usr_1")))
            .await
            .unwrap()
    );
    assert!(
        !fx.credentials
            .delete("int_1", Some(&UserId::from("usr_1")))
            .await
            .unwrap()
    );
    assert!(fx.credentials.get("int_1", None).await.unwrap().is_some());
}

async fn compare_and_swap_requires_the_expected_updated_at(fx: Fixture) {
    let fx = seeded(fx).await;
    fx.credentials
        .put(&credential("int_1", None, 10))
        .await
        .unwrap();

    let mut rotated = credential("int_1", None, 20);
    rotated.ciphertext = vec![5, 5, 5];
    assert!(
        fx.credentials
            .compare_and_swap(at(10), &rotated)
            .await
            .unwrap()
    );
    assert_eq!(
        fx.credentials.get("int_1", None).await.unwrap(),
        Some(rotated.clone())
    );

    let mut loser = credential("int_1", None, 30);
    loser.ciphertext = vec![6, 6, 6];
    assert!(
        !fx.credentials
            .compare_and_swap(at(10), &loser)
            .await
            .unwrap(),
        "stale expected_updated_at must lose"
    );
    assert_eq!(
        fx.credentials.get("int_1", None).await.unwrap(),
        Some(rotated)
    );

    let missing = credential("int_1", Some("usr_2"), 5);
    assert!(
        !fx.credentials
            .compare_and_swap(at(1), &missing)
            .await
            .unwrap()
    );
    assert!(
        fx.credentials
            .get("int_1", Some(&UserId::from("usr_2")))
            .await
            .unwrap()
            .is_none()
    );
}

async fn persisted_bytes_are_never_the_plaintext(fx: Fixture) {
    let fx = seeded(fx).await;
    let secret_box = SecretBox::from_key([3u8; 32]);
    let plaintext = "ghp_super_secret_token";
    let aad = credential_aad("int_1", None, "header");
    let sealed = secret_box
        .seal(&assistant_core::Secret::new(plaintext), aad.as_bytes())
        .unwrap();
    let stored = StoredCredential {
        integration_id: "int_1".into(),
        subject_user_id: None,
        kind: CredentialKind::Header,
        ciphertext: sealed.ciphertext.clone(),
        nonce: sealed.nonce.clone(),
        key_id: sealed.key_id.clone(),
        expires_at: None,
        status: CredentialStatus::Ready,
        updated_at: at(1),
    };
    fx.credentials.put(&stored).await.unwrap();

    let got = fx.credentials.get("int_1", None).await.unwrap().unwrap();
    assert_ne!(got.ciphertext, plaintext.as_bytes());
    assert!(
        !got.ciphertext
            .windows(plaintext.len())
            .any(|w| w == plaintext.as_bytes()),
        "plaintext leaked into the stored ciphertext"
    );
    let opened = secret_box
        .open(
            &Sealed {
                ciphertext: got.ciphertext,
                nonce: got.nonce,
                key_id: got.key_id,
            },
            aad.as_bytes(),
        )
        .unwrap();
    assert_eq!(opened.expose(), plaintext);

    if let Some(layer) = &fx.layer {
        let blob: Vec<u8> = sqlx::query_scalar("SELECT ciphertext FROM integration_credentials")
            .fetch_one(layer.pool())
            .await
            .unwrap();
        assert!(
            !blob
                .windows(plaintext.len())
                .any(|w| w == plaintext.as_bytes())
        );
    }
}

macro_rules! contract_tests {
    ($mod_name:ident, $ctor:expr) => {
        mod $mod_name {
            use super::*;

            #[tokio::test]
            async fn t_subjects_independent() {
                shared_and_per_user_subjects_are_independent($ctor).await;
            }
            #[tokio::test]
            async fn t_put_replaces() {
                put_replaces_the_existing_subject_row($ctor).await;
            }
            #[tokio::test]
            async fn t_delete_subject() {
                delete_removes_only_that_subject($ctor).await;
            }
            #[tokio::test]
            async fn t_cas() {
                compare_and_swap_requires_the_expected_updated_at($ctor).await;
            }
            #[tokio::test]
            async fn t_never_plaintext() {
                persisted_bytes_are_never_the_plaintext($ctor).await;
            }
        }
    };
}

contract_tests!(sqlite_impl, sqlite().await);
contract_tests!(memory_impl, memory());
