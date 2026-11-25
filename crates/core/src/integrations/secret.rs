//! [`SecretBox`]: AES-256-GCM encryption for integration credentials.

use std::io::Write;
use std::path::Path;

use aes_gcm::{AeadInOut, Aes256Gcm, Key, Nonce, aead::KeyInit};
use anyhow::{Context, Result, anyhow};
use base64::Engine;
use base64::engine::general_purpose::STANDARD;
use sha2::{Digest, Sha256};
use tracing::{info, warn};

use crate::identity::OrgId;
use crate::secret::Secret;

const KEY_LEN: usize = 32;
const NONCE_LEN: usize = 12;
const KEY_ID_BYTES: usize = 8;

/// File name of the generated key, relative to the assistant base directory.
pub const KEY_FILE_NAME: &str = "secrets.key";

/// Environment variable that overrides the key file (base64, 32 bytes).
pub const KEY_ENV_VAR: &str = "ASSISTANT_SECRETS_KEY";

/// An encrypted value plus what is needed to decrypt it under the right key.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Sealed {
    pub ciphertext: Vec<u8>,
    pub nonce: Vec<u8>,
    pub key_id: String,
}

/// Why a [`SecretBox::open`] call failed in a way callers handle specially.
#[derive(Debug, PartialEq, Eq)]
pub enum SecretBoxError {
    /// The value was sealed under a different key.
    KeyMismatch { stored: String, current: String },
}

impl std::fmt::Display for SecretBoxError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::KeyMismatch { stored, current } => write!(
                f,
                "secret was sealed under key {stored} but the server key is {current}"
            ),
        }
    }
}

impl std::error::Error for SecretBoxError {}

/// Authenticated encryption under one server key.
pub struct SecretBox {
    cipher: Aes256Gcm,
    key_id: String,
}

impl std::fmt::Debug for SecretBox {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("SecretBox")
            .field("key_id", &self.key_id)
            .finish_non_exhaustive()
    }
}

impl SecretBox {
    pub fn from_key(key: [u8; KEY_LEN]) -> Self {
        let digest = Sha256::digest(key);
        let key: &Key<Aes256Gcm> = (&key).into();
        Self {
            cipher: Aes256Gcm::new(key),
            key_id: hex::encode(&digest[..KEY_ID_BYTES]),
        }
    }

    /// Resolve the server key.
    ///
    /// A non-blank `env_key` (base64, 32 bytes) wins and is never persisted.
    /// Otherwise `<base_dir>/secrets.key` is read, or generated with mode
    /// `0600` on first use. An invalid `env_key` is an error rather than a
    /// fallback, so a typo cannot silently switch keys.
    pub fn load(base_dir: &Path, env_key: Option<&str>) -> Result<Self> {
        if let Some(raw) = env_key.map(str::trim).filter(|v| !v.is_empty()) {
            let key = decode_key(raw).with_context(|| format!("invalid {KEY_ENV_VAR}"))?;
            return Ok(Self::from_key(key));
        }
        let path = base_dir.join(KEY_FILE_NAME);
        let key = match read_key_file(&path)? {
            Some(key) => key,
            None => create_key_file(&path)?,
        };
        Ok(Self::from_key(key))
    }

    /// [`SecretBox::load`] with the key taken from the process environment.
    pub fn load_from_env(base_dir: &Path) -> Result<Self> {
        Self::load(base_dir, std::env::var(KEY_ENV_VAR).ok().as_deref())
    }

    /// Identifies the key (first 8 bytes of SHA-256, hex). Safe to store and log.
    pub fn key_id(&self) -> &str {
        &self.key_id
    }

    /// Encrypt `plaintext`, binding it to `aad`.
    pub fn seal(&self, plaintext: &Secret, aad: &[u8]) -> Result<Sealed> {
        let mut nonce = [0u8; NONCE_LEN];
        getrandom::fill(&mut nonce).map_err(|e| anyhow!("generating nonce: {e}"))?;
        let mut buf = plaintext.expose().as_bytes().to_vec();
        self.cipher
            .encrypt_in_place(&Nonce::from(nonce), aad, &mut buf)
            .map_err(|_| anyhow!("encrypting secret"))?;
        Ok(Sealed {
            ciphertext: buf,
            nonce: nonce.to_vec(),
            key_id: self.key_id.clone(),
        })
    }

    /// Decrypt `sealed`. Fails with [`SecretBoxError::KeyMismatch`] before any
    /// decryption when it was sealed under a different key.
    pub fn open(&self, sealed: &Sealed, aad: &[u8]) -> Result<Secret> {
        if sealed.key_id != self.key_id {
            return Err(SecretBoxError::KeyMismatch {
                stored: sealed.key_id.clone(),
                current: self.key_id.clone(),
            }
            .into());
        }
        let nonce: [u8; NONCE_LEN] = sealed.nonce.as_slice().try_into().map_err(|_| {
            anyhow!(
                "stored nonce has {} bytes, expected {NONCE_LEN}",
                sealed.nonce.len()
            )
        })?;
        let mut buf = sealed.ciphertext.clone();
        self.cipher
            .decrypt_in_place(&Nonce::from(nonce), aad, &mut buf)
            .map_err(|_| anyhow!("decrypting secret: authentication failed"))?;
        let text = String::from_utf8(buf).map_err(|_| anyhow!("decrypted secret is not UTF-8"))?;
        Ok(Secret::new(text))
    }
}

/// The AAD that binds a ciphertext to its integration, subject and kind.
///
/// `v1|<len>:<integration>|s|<kind>` for the shared credential and
/// `v1|<len>:<integration>|u<len>:<user>|<kind>` for a user's. The subject type
/// tag and the length prefixes make distinct inputs map to distinct AADs, so a
/// user named `shared`, or ids containing `|`, cannot collide.
pub fn credential_aad(integration_id: &str, subject: Option<&str>, kind: &str) -> String {
    let subject = match subject {
        None => "s".to_owned(),
        Some(user) => format!("u{}:{user}", user.len()),
    };
    format!(
        "v1|{}:{integration_id}|{subject}|{kind}",
        integration_id.len()
    )
}

/// The AAD that binds an org's OIDC client secret to that org.
///
/// `v1|oidc-client-secret|<len>:<org>`. The second field is not numeric, so it
/// cannot collide with a [`credential_aad`] value.
pub fn oidc_client_secret_aad(org_id: &OrgId) -> String {
    format!("v1|oidc-client-secret|{}:{org_id}", org_id.0.len())
}

fn decode_key(raw: &str) -> Result<[u8; KEY_LEN]> {
    let bytes = STANDARD
        .decode(raw.trim())
        .map_err(|_| anyhow!("key is not valid base64"))?;
    bytes
        .try_into()
        .map_err(|b: Vec<u8>| anyhow!("key must be {KEY_LEN} bytes, got {}", b.len()))
}

fn read_key_file(path: &Path) -> Result<Option<[u8; KEY_LEN]>> {
    let text = match std::fs::read_to_string(path) {
        Ok(text) => text,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(None),
        Err(e) => {
            return Err(e).with_context(|| format!("reading secrets key from {}", path.display()));
        }
    };
    let what = if text.trim().is_empty() {
        "empty"
    } else {
        "not a valid base64 32-byte key"
    };
    // Never regenerate: a new key would make every stored credential unreadable.
    decode_key(&text).map(Some).map_err(|_| {
        anyhow!(
            "secrets key file {} is {what}. Restore it from a backup; if no credentials have \
             been stored yet, delete the file to generate a new key",
            path.display()
        )
    })
}

/// Create the key file with all-or-nothing visibility: write and fsync a
/// private temp file in the same directory, then hard-link it into place. A
/// reader never sees a partial key, and a crash leaves at most a stray temp
/// file. If another process linked first, use its key.
fn create_key_file(path: &Path) -> Result<[u8; KEY_LEN]> {
    let dir = path
        .parent()
        .filter(|p| !p.as_os_str().is_empty())
        .unwrap_or_else(|| Path::new("."));
    std::fs::create_dir_all(dir)
        .with_context(|| format!("creating directory for {}", dir.display()))?;
    let mut key = [0u8; KEY_LEN];
    getrandom::fill(&mut key).map_err(|e| anyhow!("generating secrets key: {e}"))?;
    let mut suffix = [0u8; 8];
    getrandom::fill(&mut suffix).map_err(|e| anyhow!("generating temp name: {e}"))?;
    let tmp = dir.join(format!(".{KEY_FILE_NAME}.{}.tmp", hex::encode(suffix)));

    let written = write_private_file(&tmp, format!("{}\n", STANDARD.encode(key)).as_bytes())
        .with_context(|| format!("writing secrets key to {}", tmp.display()));
    let linked = written.and_then(|()| match std::fs::hard_link(&tmp, path) {
        Ok(()) => Ok(true),
        Err(e) if e.kind() == std::io::ErrorKind::AlreadyExists => Ok(false),
        Err(e) => Err(e).with_context(|| format!("creating secrets key {}", path.display())),
    });
    if let Err(e) = std::fs::remove_file(&tmp) {
        warn!(path = %tmp.display(), error = %e, "could not remove temp secrets key file");
    }

    if linked? {
        sync_dir(dir);
        info!(path = %path.display(), "generated integration secrets key");
        Ok(key)
    } else {
        read_key_file(path)?
            .ok_or_else(|| anyhow!("secrets key {} vanished during creation", path.display()))
    }
}

/// Create `path` exclusively with mode `0600` (Unix), write `bytes` and fsync.
fn write_private_file(path: &Path, bytes: &[u8]) -> std::io::Result<()> {
    let mut options = std::fs::OpenOptions::new();
    options.write(true).create_new(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        options.mode(0o600);
    }
    let mut file = options.open(path)?;
    file.write_all(bytes)?;
    file.sync_all()
}

/// Best-effort fsync of a directory so the new entry survives a crash.
fn sync_dir(dir: &Path) {
    #[cfg(unix)]
    if let Err(e) = std::fs::File::open(dir).and_then(|d| d.sync_all()) {
        warn!(path = %dir.display(), error = %e, "could not fsync secrets key directory");
    }
    #[cfg(not(unix))]
    let _ = dir;
}

#[cfg(test)]
mod tests {
    use super::*;

    fn key(byte: u8) -> [u8; 32] {
        [byte; 32]
    }

    #[test]
    fn round_trip() {
        let sb = SecretBox::from_key(key(1));
        let aad = credential_aad("int_1", None, "header");
        let sealed = sb.seal(&Secret::new("ghp_abc"), aad.as_bytes()).unwrap();
        let opened = sb.open(&sealed, aad.as_bytes()).unwrap();
        assert_eq!(opened.expose(), "ghp_abc");
    }

    #[test]
    fn ciphertext_does_not_contain_plaintext_and_nonce_is_random() {
        let sb = SecretBox::from_key(key(1));
        let a = sb.seal(&Secret::new("ghp_abc"), b"aad").unwrap();
        let b = sb.seal(&Secret::new("ghp_abc"), b"aad").unwrap();
        assert_eq!(a.nonce.len(), 12, "96-bit nonce");
        assert_ne!(a.nonce, b.nonce, "nonce must differ per encryption");
        assert_ne!(a.ciphertext, b.ciphertext);
        let needle = b"ghp_abc";
        assert!(
            !a.ciphertext.windows(needle.len()).any(|w| w == needle),
            "plaintext must not appear in ciphertext"
        );
    }

    #[test]
    fn aad_mismatch_fails() {
        let sb = SecretBox::from_key(key(1));
        let sealed = sb
            .seal(
                &Secret::new("v"),
                credential_aad("int_1", None, "header").as_bytes(),
            )
            .unwrap();
        let other = credential_aad("int_2", None, "header");
        assert!(sb.open(&sealed, other.as_bytes()).is_err());
    }

    #[test]
    fn credential_aad_has_a_versioned_length_prefixed_format() {
        assert_eq!(
            credential_aad("int_1", None, "header"),
            "v1|5:int_1|s|header"
        );
        assert_eq!(
            credential_aad("int_1", Some("usr_1"), "oauth"),
            "v1|5:int_1|u5:usr_1|oauth"
        );
    }

    #[test]
    fn credential_aad_is_unambiguous() {
        let cases = [
            credential_aad("int_1", None, "header"),
            credential_aad("int_1", Some("shared"), "header"),
            credential_aad("int_1", Some("usr_1"), "header"),
            credential_aad("int_1", Some("usr_1"), "oauth"),
            credential_aad("int_1|usr_1", None, "header"),
            credential_aad("int_1", Some("usr_1|header"), "oauth"),
            credential_aad("int_1|u5:usr_1", None, "header"),
            credential_aad("", Some("int_1"), "header"),
            credential_aad("int_1", Some(""), "header"),
        ];
        let unique: std::collections::HashSet<_> = cases.iter().collect();
        assert_eq!(unique.len(), cases.len(), "AAD collision among {cases:?}");
    }

    #[test]
    fn oidc_client_secret_aad_binds_the_org_and_never_collides_with_a_credential() {
        let org = OrgId::from("org_1");
        assert_eq!(
            oidc_client_secret_aad(&org),
            "v1|oidc-client-secret|5:org_1"
        );
        assert_ne!(
            oidc_client_secret_aad(&OrgId::from("org_2")),
            oidc_client_secret_aad(&org),
            "another org must not share an AAD"
        );
        let sb = SecretBox::from_key(key(1));
        let sealed = sb
            .seal(&Secret::new("v"), oidc_client_secret_aad(&org).as_bytes())
            .unwrap();
        let as_credential = credential_aad("org_1", None, "header");
        assert!(
            sb.open(&sealed, as_credential.as_bytes()).is_err(),
            "an OIDC secret must not open as an integration credential"
        );
    }

    #[test]
    fn a_ciphertext_for_the_shared_subject_does_not_open_for_a_user_named_shared() {
        let sb = SecretBox::from_key(key(1));
        let sealed = sb
            .seal(
                &Secret::new("v"),
                credential_aad("int_1", None, "header").as_bytes(),
            )
            .unwrap();
        let as_user = credential_aad("int_1", Some("shared"), "header");
        assert!(sb.open(&sealed, as_user.as_bytes()).is_err());
    }

    #[test]
    fn wrong_key_is_detected_via_key_id_without_decrypting() {
        let sb1 = SecretBox::from_key(key(1));
        let sb2 = SecretBox::from_key(key(2));
        assert_ne!(sb1.key_id(), sb2.key_id());
        let sealed = sb1.seal(&Secret::new("v"), b"aad").unwrap();
        let err = sb2.open(&sealed, b"aad").unwrap_err();
        let mismatch = err.downcast_ref::<SecretBoxError>();
        assert!(
            matches!(mismatch, Some(SecretBoxError::KeyMismatch { .. })),
            "expected KeyMismatch, got {err:#}"
        );
    }

    #[test]
    fn key_id_is_16_hex_chars_and_stable() {
        let sb = SecretBox::from_key(key(1));
        assert_eq!(sb.key_id().len(), 16);
        assert!(sb.key_id().chars().all(|c| c.is_ascii_hexdigit()));
        assert_eq!(sb.key_id(), SecretBox::from_key(key(1)).key_id());
    }

    #[test]
    fn errors_and_debug_never_contain_key_material() {
        let k = key(7);
        let sb = SecretBox::from_key(k);
        let sealed = SecretBox::from_key(key(8))
            .seal(&Secret::new("v"), b"aad")
            .unwrap();
        let err = format!("{:#}", sb.open(&sealed, b"aad").unwrap_err());
        let b64 = STANDARD.encode(k);
        for text in [err, format!("{sb:?}")] {
            assert!(!text.contains(&b64), "base64 key leaked: {text}");
            assert!(!text.contains(&hex::encode(k)), "hex key leaked: {text}");
        }
    }

    #[test]
    fn tampered_ciphertext_fails() {
        let sb = SecretBox::from_key(key(1));
        let mut sealed = sb.seal(&Secret::new("v"), b"aad").unwrap();
        sealed.ciphertext[0] ^= 0xff;
        assert!(sb.open(&sealed, b"aad").is_err());
    }

    #[test]
    fn load_generates_key_file_once() {
        let dir = tempfile::tempdir().unwrap();
        let first = SecretBox::load(dir.path(), None).unwrap();
        let path = dir.path().join(KEY_FILE_NAME);
        assert!(path.exists(), "key file created on first use");
        let second = SecretBox::load(dir.path(), None).unwrap();
        assert_eq!(first.key_id(), second.key_id(), "same key reloaded");
    }

    #[cfg(unix)]
    #[test]
    fn generated_key_file_is_mode_0600() {
        use std::os::unix::fs::PermissionsExt;
        let dir = tempfile::tempdir().unwrap();
        SecretBox::load(dir.path(), None).unwrap();
        let mode = std::fs::metadata(dir.path().join(KEY_FILE_NAME))
            .unwrap()
            .permissions()
            .mode();
        assert_eq!(mode & 0o777, 0o600);
    }

    #[test]
    fn env_key_overrides_file_and_creates_nothing() {
        let dir = tempfile::tempdir().unwrap();
        let from_file = SecretBox::load(dir.path(), None).unwrap();
        let env = STANDARD.encode(key(9));
        let from_env = SecretBox::load(dir.path(), Some(&env)).unwrap();
        assert_eq!(from_env.key_id(), SecretBox::from_key(key(9)).key_id());
        assert_ne!(from_env.key_id(), from_file.key_id());

        let empty = tempfile::tempdir().unwrap();
        SecretBox::load(empty.path(), Some(&env)).unwrap();
        assert!(!empty.path().join(KEY_FILE_NAME).exists());
    }

    #[test]
    fn invalid_env_key_is_rejected_not_ignored() {
        let dir = tempfile::tempdir().unwrap();
        assert!(SecretBox::load(dir.path(), Some("not base64!!")).is_err());
        let short = STANDARD.encode([1u8; 16]);
        assert!(SecretBox::load(dir.path(), Some(&short)).is_err());
        assert!(
            !dir.path().join(KEY_FILE_NAME).exists(),
            "a bad env key must not silently fall back to a file"
        );
    }

    #[test]
    fn blank_env_key_falls_back_to_file() {
        let dir = tempfile::tempdir().unwrap();
        SecretBox::load(dir.path(), Some("  ")).unwrap();
        assert!(dir.path().join(KEY_FILE_NAME).exists());
    }

    #[test]
    fn corrupt_key_file_is_an_error() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join(KEY_FILE_NAME), "garbage").unwrap();
        assert!(SecretBox::load(dir.path(), None).is_err());
    }

    #[test]
    fn empty_key_file_is_a_clear_error_and_is_never_regenerated() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join(KEY_FILE_NAME);
        std::fs::write(&path, "").unwrap();
        let err = format!("{:#}", SecretBox::load(dir.path(), None).unwrap_err());
        assert!(err.contains("empty"), "should say the file is empty: {err}");
        assert!(
            err.contains(&path.display().to_string()),
            "should name the file: {err}"
        );
        assert!(
            err.contains("backup") || err.contains("delete"),
            "should say what to do: {err}"
        );
        assert_eq!(
            std::fs::read(&path).unwrap(),
            b"",
            "file must stay untouched"
        );
    }

    #[test]
    fn truncated_key_file_is_a_clear_error_and_is_never_regenerated() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join(KEY_FILE_NAME);
        let full = STANDARD.encode(key(5));
        std::fs::write(&path, &full[..20]).unwrap();
        let err = format!("{:#}", SecretBox::load(dir.path(), None).unwrap_err());
        assert!(err.contains(&path.display().to_string()), "{err}");
        assert!(err.contains("backup") || err.contains("delete"), "{err}");
        assert_eq!(std::fs::read_to_string(&path).unwrap(), &full[..20]);
    }

    #[test]
    fn key_creation_leaves_no_temp_files() {
        let dir = tempfile::tempdir().unwrap();
        SecretBox::load(dir.path(), None).unwrap();
        let names: Vec<_> = std::fs::read_dir(dir.path())
            .unwrap()
            .map(|e| e.unwrap().file_name().to_string_lossy().into_owned())
            .collect();
        assert_eq!(names, [KEY_FILE_NAME]);
    }

    #[test]
    fn concurrent_first_use_agrees_on_one_key() {
        for _ in 0..30 {
            let dir = tempfile::tempdir().unwrap();
            let base = dir.path().to_path_buf();
            let handles: Vec<_> = (0..8)
                .map(|_| {
                    let base = base.clone();
                    std::thread::spawn(move || {
                        SecretBox::load(&base, None).map(|sb| sb.key_id().to_owned())
                    })
                })
                .collect();
            let ids: Vec<String> = handles
                .into_iter()
                .map(|h| h.join().unwrap().unwrap())
                .collect();
            assert!(
                ids.windows(2).all(|w| w[0] == w[1]),
                "key ids differ: {ids:?}"
            );
        }
    }
}
