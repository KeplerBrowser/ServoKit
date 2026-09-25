//! Webview handle and command facade exports.

pub use servokit_embedder::{NavigationError, NavigationRequest, WebViewCommand, WebViewHandle};

#[cfg(all(feature = "macos-system-webview", target_os = "macos"))]
/// macOS host adapter for create-time Servo or system-WebView selection.
pub mod macos;
