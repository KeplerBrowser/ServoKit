use rustls::crypto::{self, CryptoProvider};

/// Result of attempting to install a process-wide rustls crypto provider.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RustlsCryptoProviderStatus {
    /// Servokit installed a provider for this process.
    Installed,
    /// A provider was already installed, so Servokit left it unchanged.
    AlreadyInstalled,
}

/// Ensure Servokit's default rustls crypto provider is installed.
///
/// Servokit defaults to rustls' `aws_lc_rs` provider to preserve the current desktop example
/// behavior, except on Android where Servokit enables `ring` because `aws_lc_rs` has been
/// observed to crash during TLS key generation. Embedding applications that need a different
/// provider should call [`install_rustls_crypto_provider`] before creating any Servo-backed
/// surfaces or webviews.
#[must_use]
pub fn ensure_default_rustls_crypto_provider() -> RustlsCryptoProviderStatus {
    install_rustls_crypto_provider(servokit_default_rustls_crypto_provider())
}

#[cfg(target_os = "android")]
fn servokit_default_rustls_crypto_provider() -> CryptoProvider {
    crypto::ring::default_provider()
}

#[cfg(not(target_os = "android"))]
fn servokit_default_rustls_crypto_provider() -> CryptoProvider {
    crypto::aws_lc_rs::default_provider()
}

/// Install an app-supplied process-wide rustls crypto provider if one is not already installed.
#[must_use]
pub fn install_rustls_crypto_provider(provider: CryptoProvider) -> RustlsCryptoProviderStatus {
    if CryptoProvider::get_default().is_some() {
        return RustlsCryptoProviderStatus::AlreadyInstalled;
    }

    match provider.install_default() {
        Ok(()) => RustlsCryptoProviderStatus::Installed,
        Err(_) => RustlsCryptoProviderStatus::AlreadyInstalled,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[cfg(target_os = "android")]
    fn expected_default_provider() -> CryptoProvider {
        crypto::ring::default_provider()
    }

    #[cfg(not(target_os = "android"))]
    fn expected_default_provider() -> CryptoProvider {
        crypto::aws_lc_rs::default_provider()
    }

    #[test]
    fn default_provider_uses_expected_platform_provider() {
        let provider = servokit_default_rustls_crypto_provider();
        let expected = expected_default_provider();
        assert_eq!(provider.kx_groups.len(), expected.kx_groups.len());
        assert!(std::ptr::addr_eq(
            provider.kx_groups[0],
            expected.kx_groups[0]
        ));
    }

    #[test]
    fn default_provider_is_installed_once_and_then_reused() {
        let status = ensure_default_rustls_crypto_provider();
        assert!(matches!(
            status,
            RustlsCryptoProviderStatus::Installed | RustlsCryptoProviderStatus::AlreadyInstalled
        ));
        assert!(CryptoProvider::get_default().is_some());
        assert_eq!(
            install_rustls_crypto_provider(servokit_default_rustls_crypto_provider()),
            RustlsCryptoProviderStatus::AlreadyInstalled
        );
    }
}
