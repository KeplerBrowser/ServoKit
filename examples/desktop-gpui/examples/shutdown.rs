//! Real GPUI close/quit regression for final Servo shutdown before logging TLS destruction.
//! Run with RUST_LOG=warn; --retain-runtime is the intentionally failing legacy control.
#![allow(deprecated, unexpected_cfgs)]

#[allow(dead_code)]
#[path = "../src/appkit_surface.rs"]
mod appkit_surface;

use std::{
    rc::Rc,
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc,
    },
    thread::{self, ThreadId},
    time::{Duration, Instant},
};

use appkit_surface::{
    new_surface_state, store_surface_state, viewport, GpuiServoSurface, SharedSurfaceState,
};
use cocoa::base::{id, nil};
use objc::{msg_send, sel, sel_impl};
use raw_window_handle::{HasWindowHandle, RawWindowHandle};
use servokit::{
    events::{HostEvent, LoadStatusKind},
    runtime::Runtime,
    surface::{HostSurface, MemoryClipboard, SurfaceHost, SurfaceHostOptions},
    webview::WebViewHandle,
};
use zed_gpui::*;

type BrowserRuntime = Runtime<SurfaceHost<GpuiServoSurface>>;

#[derive(Clone, Copy, Debug, PartialEq)]
enum ExitPath {
    CloseWindow,
    AppQuit,
}

struct ShutdownProof {
    surface: SharedSurfaceState,
    runtime: Option<BrowserRuntime>,
    webview: Option<WebViewHandle>,
    update_task: Option<Task<()>>,
    wake_retired: Arc<AtomicBool>,
    owner_thread: ThreadId,
    native_window: id,
    path: ExitPath,
    retain_runtime: bool,
    unattached_replacement: bool,
    logger_after_page: bool,
    requested_exit: bool,
    title_seen: bool,
    load_complete: bool,
    started: Instant,
}

impl ShutdownProof {
    fn new(
        native_window: id,
        path: ExitPath,
        retain_runtime: bool,
        unattached_replacement: bool,
        logger_after_page: bool,
        cx: &mut Context<Self>,
    ) -> Self {
        cx.on_app_quit(|this, _| {
            this.finish("application-quit");
            async {}
        })
        .detach();
        let update_task = cx.spawn(async move |this, cx| loop {
            cx.background_executor()
                .timer(Duration::from_millis(16))
                .await;
            if this
                .update(cx, |this, cx| {
                    this.pump(cx);
                    cx.notify();
                })
                .is_err()
            {
                break;
            }
        });
        Self {
            surface: new_surface_state(),
            runtime: None,
            webview: None,
            update_task: Some(update_task),
            wake_retired: Arc::new(AtomicBool::new(false)),
            owner_thread: thread::current().id(),
            native_window,
            path,
            retain_runtime,
            unattached_replacement,
            logger_after_page,
            requested_exit: false,
            title_seen: false,
            load_complete: false,
            started: Instant::now(),
        }
    }

    fn pump(&mut self, cx: &mut Context<Self>) {
        assert!(
            !self.wake_retired.load(Ordering::Acquire),
            "update task ran after shutdown"
        );
        assert!(
            self.started.elapsed() < Duration::from_secs(30),
            "shutdown proof timed out"
        );
        if self.runtime.is_none() {
            let Some(viewport) = viewport(&self.surface) else {
                return;
            };
            let wake_retired = self.wake_retired.clone();
            let mut runtime = Runtime::new(SurfaceHost::new(
                GpuiServoSurface::new(self.surface.clone()),
                SurfaceHostOptions::new(
                    Arc::new(move || {
                        assert!(
                            !wake_retired.load(Ordering::Acquire),
                            "wake target used after retirement"
                        );
                    }),
                    Rc::new(MemoryClipboard::default()),
                ),
            ));
            let session = runtime.create_session();
            let webview = runtime.create_webview(session).unwrap();
            runtime.load_url(webview, "data:text/html,<title>Shutdown</title><h1>Live Servo page</h1><script>setInterval(()=>window.tick=(window.tick||0)+1,10)</script>").unwrap();
            runtime
                .attach_surface_with_viewport(webview, HostSurface::new("shutdown-proof"), viewport)
                .unwrap();
            self.webview = Some(webview);
            self.runtime = Some(runtime);
        }
        let runtime = self.runtime.as_mut().unwrap();
        runtime.perform_all_updates().unwrap();
        for event in runtime.drain_events() {
            match event.event {
                HostEvent::NavigationRequested { navigation_id, .. } => {
                    runtime
                        .resolve_navigation_request(event.webview, &navigation_id, true)
                        .unwrap();
                }
                HostEvent::PageTitleChanged { title } => {
                    self.title_seen = title.as_deref() == Some("Shutdown")
                }
                HostEvent::LoadStatusChanged { status } => {
                    self.load_complete = status == LoadStatusKind::Complete
                }
                HostEvent::Error { message, .. } => panic!("Servo error: {message}"),
                HostEvent::Crashed { reason, .. } => panic!("Servo crashed: {reason}"),
                _ => {}
            }
        }
        if self.load_complete && self.title_seen && !self.requested_exit {
            self.requested_exit = true;
            if self.logger_after_page {
                init_logging();
            }
            println!(
                "shutdown-proof live-page path={:?} retained={} late-logger={}",
                self.path, self.retain_runtime, self.logger_after_page
            );
            match self.path {
                ExitPath::AppQuit => cx.quit(),
                ExitPath::CloseWindow => unsafe {
                    // Schedule AppKit's real close selector outside GPUI's app-state borrow.
                    let _: () = msg_send![self.native_window,
                        performSelector: sel!(performClose:) withObject: nil afterDelay: 0.0f64];
                },
            }
        }
    }

    fn finish(&mut self, callback: &str) {
        assert_eq!(thread::current().id(), self.owner_thread);
        let Some(runtime) = self.runtime.take() else {
            return;
        };
        assert!(
            self.webview.take().is_some(),
            "shutdown must start with a live page"
        );
        assert!(self.load_complete && self.title_seen);
        drop(self.update_task.take()); // GPUI Task drop cancels further update work.
        println!(
            "shutdown-proof {callback}: task-cancelled; surface-alive={}",
            viewport(&self.surface).is_some()
        );
        if self.unattached_replacement {
            drop(runtime);
            Runtime::new(SurfaceHost::new(
                GpuiServoSurface::new(self.surface.clone()),
                SurfaceHostOptions::new(Arc::new(|| {}), Rc::new(MemoryClipboard::default())),
            ))
            .shutdown()
            .unwrap();
            println!("shutdown-proof unattached-replacement-finalized");
        } else if self.retain_runtime {
            drop(runtime);
        } else {
            runtime.shutdown().unwrap();
        }
        assert_eq!(
            Arc::strong_count(&self.wake_retired),
            1,
            "engine retained a wake target"
        );
        self.wake_retired.store(true, Ordering::Release);
        let disposition = if self.retain_runtime {
            "runtime-retained"
        } else {
            "runtime-finalized"
        };
        println!(
            "shutdown-proof {callback}: {disposition}; wake-retired; surface-alive={}",
            viewport(&self.surface).is_some()
        );
    }
}

impl Drop for ShutdownProof {
    fn drop(&mut self) {
        assert!(
            self.runtime.is_none(),
            "host dropped before shutdown callback"
        );
        assert!(
            self.update_task.is_none(),
            "update task survived host teardown"
        );
        println!("shutdown-proof host-destructor");
    }
}

impl Render for ShutdownProof {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        let surface = self.surface.clone();
        div()
            .size_full()
            .on_children_prepainted(move |bounds, window, _| {
                if let Some(bounds) = bounds.first().copied() {
                    store_surface_state(&surface, bounds, window);
                }
            })
            .child(div().size_full())
    }
}

fn init_logging() {
    // Subscriber init also installs tracing-log's LogTracer. Do not emit a warmup event.
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env().unwrap_or_else(|_| "warn".into()),
        )
        .with_ansi(false)
        .init();
}

fn main() {
    let args: Vec<_> = std::env::args().skip(1).collect();
    assert!(args.iter().all(|arg| matches!(
        arg.as_str(),
        "--close-window"
            | "--app-quit"
            | "--retain-runtime"
            | "--logger-after-page"
            | "--unattached-replacement"
    )));
    let path = if args.iter().any(|arg| arg == "--close-window") {
        ExitPath::CloseWindow
    } else {
        ExitPath::AppQuit
    };
    let retain_runtime = args.iter().any(|arg| arg == "--retain-runtime");
    let unattached_replacement = args.iter().any(|arg| arg == "--unattached-replacement");
    assert!(!(retain_runtime && unattached_replacement));
    let logger_after_page = args.iter().any(|arg| arg == "--logger-after-page");
    if !logger_after_page {
        init_logging();
    }
    Application::new().run(move |cx: &mut App| {
        cx.on_window_closed(move |cx| {
            println!("shutdown-proof native-window-closed");
            if path == ExitPath::CloseWindow {
                cx.quit();
            }
        })
        .detach();
        cx.open_window(
            WindowOptions {
                window_bounds: Some(WindowBounds::Windowed(Bounds::centered(
                    None,
                    size(px(640.0), px(480.0)),
                    cx,
                ))),
                ..Default::default()
            },
            move |window, cx| {
                let RawWindowHandle::AppKit(handle) =
                    HasWindowHandle::window_handle(window).unwrap().as_raw()
                else {
                    panic!("macOS AppKit required")
                };
                let native_window: id = unsafe { msg_send![handle.ns_view.as_ptr() as id, window] };
                let proof = cx.new(|cx| {
                    ShutdownProof::new(
                        native_window,
                        path,
                        retain_runtime,
                        unattached_replacement,
                        logger_after_page,
                        cx,
                    )
                });
                let weak = proof.downgrade();
                window.on_window_should_close(cx, move |_, cx| {
                    weak.update(cx, |this, _| this.finish("window-close"))
                        .unwrap();
                    true
                });
                proof
            },
        )
        .unwrap();
        cx.activate(true);
    });
}
