use servokit::events::{HostEvent, ServokitEvent};

pub fn print(event: &ServokitEvent) {
    match &event.event {
        HostEvent::LoadStatusChanged { status } => {
            println!("webview={} load={}", event.webview.raw(), status.as_str())
        }
        HostEvent::UrlChanged { url } => println!("webview={} url={url}", event.webview.raw()),
        HostEvent::SurfaceAttached { size } => println!(
            "webview={} surface attached {}x{}",
            event.webview.raw(),
            size.width,
            size.height
        ),
        HostEvent::SurfaceResized { size } => println!(
            "webview={} surface resized {}x{}",
            event.webview.raw(),
            size.width,
            size.height
        ),
        HostEvent::SurfaceDetached => println!("webview={} surface detached", event.webview.raw()),
        HostEvent::Error { url, code, message } => println!(
            "webview={} error code={} url={url:?} message={}",
            event.webview.raw(),
            code,
            message
        ),
        HostEvent::Crashed {
            url,
            reason,
            backtrace,
        } => println!(
            "webview={} crashed url={url:?} reason={} backtrace={}",
            event.webview.raw(),
            reason,
            backtrace.as_deref().unwrap_or("none")
        ),
        HostEvent::FocusChanged { is_focused } => {
            println!("webview={} focus={is_focused}", event.webview.raw())
        }
        other => println!("webview={} event={other:?}", event.webview.raw()),
    }
}
