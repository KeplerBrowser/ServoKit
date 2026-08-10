//! Runtime/session facade exports.

pub use servokit_embedder::{Runtime, RuntimeError, RuntimeError as ServokitError, SessionHandle};

#[cfg(all(
    feature = "servo",
    any(
        target_os = "android",
        target_os = "macos",
        target_os = "windows",
        target_os = "linux"
    )
))]
pub use servokit_embedder::{
    ensure_default_rustls_crypto_provider, install_rustls_crypto_provider,
    RustlsCryptoProviderStatus,
};
