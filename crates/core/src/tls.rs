//! Process-wide TLS crypto provider setup.
//!
//! The workspace compiles a single rustls backend, `ring`. HTTP clients are
//! built with reqwest's `rustls-no-provider` feature, which leaves provider
//! selection to the process, so every binary calls [`install_ring`] before it
//! opens its first TLS connection.

/// Installs `ring` as the process-wide default rustls crypto provider.
///
/// Safe to call repeatedly and from several threads. If another provider is
/// already installed, it stays in place.
pub fn install_ring() {
    static INSTALL: std::sync::Once = std::sync::Once::new();
    INSTALL.call_once(|| {
        let _ = rustls::crypto::ring::default_provider().install_default();
    });
}

/// Installs the provider before test binaries start, so tests that build a
/// reqwest client need no setup. Enabled through the `test-tls` feature.
#[cfg(feature = "test-tls")]
#[ctor::ctor(unsafe)]
fn install_ring_for_tests() {
    install_ring();
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reqwest_client_builds_after_install_ring() {
        install_ring();
        reqwest::Client::builder()
            .build()
            .expect("reqwest client should build once the ring provider is installed");
    }

    #[test]
    fn install_ring_is_idempotent_and_sets_the_default_provider() {
        install_ring();
        install_ring();
        assert!(
            rustls::crypto::CryptoProvider::get_default().is_some(),
            "a default crypto provider should be installed"
        );
    }
}
