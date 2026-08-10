mod event_log;
mod input;
mod smoke;

use std::{
    error::Error,
    rc::Rc,
    sync::{Arc, Mutex},
    time::{Duration, Instant},
};

use servokit::{
    runtime::{ensure_default_rustls_crypto_provider, Runtime},
    surface::{
        HostSurface, MemoryClipboard, NativeChildSurface, SurfaceDelegate, SurfaceError,
        SurfaceFrame, SurfaceHost, SurfaceHostOptions, SurfaceSize, SurfaceTarget, SurfaceViewport,
    },
    webview::WebViewHandle,
    HostEvent,
};
use winit::{
    application::ApplicationHandler,
    dpi::LogicalSize,
    event::WindowEvent,
    event_loop::{ActiveEventLoop, ControlFlow, EventLoop},
    raw_window_handle::{HasDisplayHandle, HasWindowHandle},
    window::{Window, WindowId},
};

const APP_TITLE: &str = "Servokit winit desktop example";
const SURFACE_ID: &str = "desktop-winit-window";
const DEFAULT_INITIAL_URL: &str = "http://127.0.0.1:8481/smoke/index.html";

type DesktopRuntime = Runtime<SurfaceHost<WinitSurface>>;

#[derive(Clone, Copy, Debug)]
enum AppEvent {
    Wake,
}

fn main() -> Result<(), Box<dyn Error>> {
    let _ = ensure_default_rustls_crypto_provider();

    let config = AppConfig::parse(std::env::args().skip(1))?;
    let event_loop = EventLoop::<AppEvent>::with_user_event().build()?;
    event_loop.set_control_flow(ControlFlow::Wait);

    let proxy = Arc::new(Mutex::new(event_loop.create_proxy()));
    let waker = Arc::new(move || {
        if let Ok(proxy) = proxy.lock() {
            let _ = proxy.send_event(AppEvent::Wake);
        }
    });
    let options = SurfaceHostOptions::new(waker, Rc::new(MemoryClipboard::default()));
    let mut app = DesktopWinitExample::new(config, options);
    event_loop.run_app(&mut app)?;
    if let Some(exit_code) = app.exit_code {
        std::process::exit(exit_code);
    }
    Ok(())
}

#[derive(Debug)]
struct AppConfig {
    initial_url: String,
    mode: RunMode,
}

#[derive(Debug)]
enum RunMode {
    Interactive,
    Smoke(smoke::SmokeOptions),
}

impl AppConfig {
    fn parse(args: impl IntoIterator<Item = String>) -> Result<Self, Box<dyn Error>> {
        let mut initial_url = None;
        let mut smoke_options = None;
        let mut args = args.into_iter();
        while let Some(arg) = args.next() {
            match arg.as_str() {
                "--help" | "-h" => {
                    print_usage();
                    std::process::exit(0);
                }
                "--smoke" => {
                    smoke_options.get_or_insert_with(smoke::SmokeOptions::default);
                }
                "--smoke-timeout-ms" => {
                    let value = args.next().ok_or_else(|| {
                        "--smoke-timeout-ms requires a timeout in milliseconds".to_owned()
                    })?;
                    smoke_options = Some(smoke::SmokeOptions {
                        timeout: parse_timeout_ms(&value)?,
                    });
                }
                _ if arg.starts_with("--smoke-timeout-ms=") => {
                    let value = arg
                        .split_once('=')
                        .map(|(_, value)| value)
                        .expect("prefix match guarantees split value");
                    smoke_options = Some(smoke::SmokeOptions {
                        timeout: parse_timeout_ms(value)?,
                    });
                }
                _ => {
                    if initial_url.replace(arg.clone()).is_some() {
                        return Err(format!("unexpected extra URL argument: {arg}").into());
                    }
                }
            }
        }

        Ok(Self {
            initial_url: initial_url.unwrap_or_else(|| DEFAULT_INITIAL_URL.to_owned()),
            mode: smoke_options
                .map(RunMode::Smoke)
                .unwrap_or(RunMode::Interactive),
        })
    }
}

fn parse_timeout_ms(value: &str) -> Result<Duration, Box<dyn Error>> {
    let timeout_ms: u64 = value.parse()?;
    if timeout_ms == 0 {
        return Err("--smoke-timeout-ms must be greater than 0".into());
    }
    Ok(Duration::from_millis(timeout_ms))
}

fn print_usage() {
    println!(
        "Usage: desktop-winit [--smoke] [--smoke-timeout-ms <ms>] [url]\n\n\
         Without --smoke, the example runs interactively and uses the optional URL.\n\
         With --smoke, the example loads the URL (default: {DEFAULT_INITIAL_URL}), prints a\n\
         deterministic platform/surface/load summary, and exits 0 on load completion or 1 on\n\
         timeout, Servo error, or crash."
    );
}

struct WinitSurface {
    window: Rc<Window>,
}

impl SurfaceDelegate for WinitSurface {
    type Frame<'a> = SurfaceFrame<'a>;

    fn render_target(
        &mut self,
        _surface: &HostSurface,
        _viewport: SurfaceViewport,
    ) -> Result<SurfaceTarget<'_>, SurfaceError> {
        let display = self
            .window
            .display_handle()
            .map_err(|error| SurfaceError::new(error.to_string()))?;
        let window = self
            .window
            .window_handle()
            .map_err(|error| SurfaceError::new(error.to_string()))?;
        Ok(NativeChildSurface::new(display, window).into())
    }
}

struct DesktopWinitExample {
    initial_url: String,
    options: SurfaceHostOptions,
    runtime: Option<DesktopRuntime>,
    window: Option<Rc<Window>>,
    webview: Option<WebViewHandle>,
    cursor: Option<input::CursorPoint>,
    smoke: Option<smoke::SmokeState>,
    exit_code: Option<i32>,
}

impl DesktopWinitExample {
    fn new(config: AppConfig, options: SurfaceHostOptions) -> Self {
        let smoke = match config.mode {
            RunMode::Interactive => None,
            RunMode::Smoke(options) => {
                Some(smoke::SmokeState::new(config.initial_url.clone(), options))
            }
        };
        Self {
            initial_url: config.initial_url,
            options,
            runtime: None,
            window: None,
            webview: None,
            cursor: None,
            smoke,
            exit_code: None,
        }
    }

    fn start(&mut self, event_loop: &ActiveEventLoop) -> Result<(), Box<dyn Error>> {
        if self.window.is_some() {
            return Ok(());
        }
        let attrs = Window::default_attributes()
            .with_title(APP_TITLE)
            .with_inner_size(LogicalSize::new(960.0, 640.0));
        let window = Rc::new(event_loop.create_window(attrs)?);
        window.set_ime_allowed(true);
        self.cursor = None;

        let host = SurfaceHost::new(
            WinitSurface {
                window: window.clone(),
            },
            self.options.clone(),
        );
        let mut runtime = Runtime::new(host);
        let session = runtime.create_session();
        let webview = runtime.create_webview(session)?;
        runtime.load_url(webview, &self.initial_url)?;
        runtime.attach_surface_with_viewport(
            webview,
            HostSurface::new(SURFACE_ID),
            viewport(&window),
        )?;
        runtime.perform_updates(webview)?;

        self.webview = Some(webview);
        self.window = Some(window);
        self.runtime = Some(runtime);
        self.drain_events();
        Ok(())
    }

    fn pump_updates(&mut self, event_loop: &ActiveEventLoop) {
        event_loop.set_control_flow(ControlFlow::WaitUntil(
            Instant::now() + Duration::from_millis(16),
        ));
        let (Some(runtime), Some(webview)) = (self.runtime.as_mut(), self.webview) else {
            return;
        };
        if let Err(error) = runtime.perform_updates(webview) {
            let message = format!("servokit update failed: {error}");
            eprintln!("{message}");
            self.record_smoke_failure(message);
            self.finish_smoke_or_exit(event_loop);
            return;
        }
        self.drain_events();
        self.finish_smoke_if_ready(event_loop);
    }

    fn drain_events(&mut self) {
        loop {
            let Some(runtime) = self.runtime.as_mut() else {
                return;
            };
            let events = runtime.drain_events();
            if events.is_empty() {
                return;
            }

            let mut navigation_requests = Vec::new();
            for event in events {
                event_log::print(&event);
                if event.managed_child_webview_id.is_none() {
                    if let HostEvent::NavigationRequested { navigation_id, .. } = &event.event {
                        navigation_requests.push((event.webview, navigation_id.clone()));
                    }
                }
                if let Some(smoke) = self.smoke.as_mut() {
                    smoke.observe(&event);
                }
            }

            let mut failures = Vec::new();
            for (webview, navigation_id) in navigation_requests {
                let Some(runtime) = self.runtime.as_mut() else {
                    return;
                };
                if let Err(error) =
                    runtime.resolve_navigation_request(webview, &navigation_id, true)
                {
                    failures.push(format!(
                        "servokit navigation request {navigation_id} resolution failed: {error}"
                    ));
                }
            }
            for failure in failures {
                eprintln!("{failure}");
                self.record_smoke_failure(failure);
            }
        }
    }

    fn update_viewport(&mut self, event_loop: &ActiveEventLoop) {
        let (Some(runtime), Some(window), Some(webview)) =
            (self.runtime.as_mut(), self.window.as_ref(), self.webview)
        else {
            return;
        };
        let viewport = viewport(window);
        if let Some(cursor) = self.cursor {
            self.cursor = Some(input::clamp_cursor_point(cursor, viewport.size));
        }
        if let Err(error) = runtime.update_surface_viewport(webview, viewport) {
            let message = format!("servokit viewport update failed: {error}");
            eprintln!("{message}");
            self.record_smoke_failure(message);
            self.finish_smoke_or_exit(event_loop);
            return;
        }
        self.drain_events();
        self.finish_smoke_if_ready(event_loop);
    }

    fn dispatch_input(&mut self, event: &WindowEvent, event_loop: &ActiveEventLoop) {
        let Some(window) = self.window.as_ref() else {
            return;
        };
        let Some(input) = input::to_servokit_event(event, &mut self.cursor, viewport(window).size)
        else {
            return;
        };
        let (Some(runtime), Some(webview)) = (self.runtime.as_mut(), self.webview) else {
            return;
        };
        if let Err(error) = runtime.dispatch_input_event(webview, input) {
            let message = format!("servokit input dispatch failed: {error}");
            eprintln!("{message}");
            self.record_smoke_failure(message);
            self.finish_smoke_or_exit(event_loop);
            return;
        }
        self.drain_events();
        self.finish_smoke_if_ready(event_loop);
    }

    fn record_smoke_failure(&mut self, reason: impl Into<String>) {
        if let Some(smoke) = self.smoke.as_mut() {
            smoke.record_failure(reason);
            self.exit_code = Some(1);
        }
    }

    fn finish_smoke_or_exit(&mut self, event_loop: &ActiveEventLoop) {
        if self.smoke.is_some() {
            self.finish_smoke_if_ready(event_loop);
        } else {
            event_loop.exit();
        }
    }

    fn finish_smoke_if_ready(&mut self, event_loop: &ActiveEventLoop) {
        if self.exit_code.is_some() && self.smoke.is_none() {
            event_loop.exit();
            return;
        }
        let outcome = self.smoke.as_ref().and_then(|smoke| smoke.outcome());
        let Some(outcome) = outcome else {
            return;
        };
        self.exit_code = Some(outcome.exit_code());
        self.detach_surface();
        if let Some(smoke) = self.smoke.take() {
            smoke.print_result(&outcome);
        }
        event_loop.exit();
    }

    fn detach_surface(&mut self) {
        let (Some(runtime), Some(webview)) = (self.runtime.as_mut(), self.webview.take()) else {
            return;
        };
        if let Err(error) = runtime.detach_surface(webview) {
            eprintln!("servokit surface detach failed: {error}");
        }
        self.drain_events();
    }
}

impl ApplicationHandler<AppEvent> for DesktopWinitExample {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        if let Err(error) = self.start(event_loop) {
            let message = format!("failed to start {APP_TITLE}: {error}");
            eprintln!("{message}");
            self.record_smoke_failure(message);
            self.finish_smoke_or_exit(event_loop);
            return;
        }
        self.finish_smoke_if_ready(event_loop);
    }

    fn user_event(&mut self, event_loop: &ActiveEventLoop, _event: AppEvent) {
        self.pump_updates(event_loop);
    }

    fn window_event(
        &mut self,
        event_loop: &ActiveEventLoop,
        window_id: WindowId,
        event: WindowEvent,
    ) {
        if self.window.as_ref().map(|window| window.id()) != Some(window_id) {
            return;
        }
        match &event {
            WindowEvent::CloseRequested | WindowEvent::Destroyed => {
                if self.smoke.is_some() && self.exit_code.is_none() {
                    self.record_smoke_failure("window closed before smoke completed");
                }
                self.detach_surface();
                self.finish_smoke_or_exit(event_loop);
            }
            WindowEvent::Resized(_) | WindowEvent::ScaleFactorChanged { .. } => {
                self.update_viewport(event_loop)
            }
            WindowEvent::RedrawRequested => self.pump_updates(event_loop),
            _ => self.dispatch_input(&event, event_loop),
        }
    }

    fn about_to_wait(&mut self, event_loop: &ActiveEventLoop) {
        self.pump_updates(event_loop);
    }
}

fn viewport(window: &Window) -> SurfaceViewport {
    let size = window.inner_size();
    SurfaceViewport::new(
        SurfaceSize::new(size.width, size.height),
        window.scale_factor() as f32,
    )
}
