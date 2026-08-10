mod appkit_surface;
mod input;

use std::{rc::Rc, sync::Arc, time::Duration};

use appkit_surface::{
    new_surface_state, store_surface_state, take_last_error, viewport, GpuiServoSurface,
    SharedSurfaceState,
};
use servokit::{
    events::HostEvent,
    input::HostInputEvent,
    runtime::{ensure_default_rustls_crypto_provider, Runtime},
    surface::{HostSurface, MemoryClipboard, SurfaceHost, SurfaceHostOptions, SurfaceViewport},
    webview::WebViewHandle,
};
use zed_gpui::*;

const SURFACE_ID: &str = "desktop-gpui-layout-slot";
const DEFAULT_INITIAL_URL: &str = "http://127.0.0.1:8481/smoke/index.html";

type GpuiRuntime = Runtime<SurfaceHost<GpuiServoSurface>>;

struct ServoGpuiExample {
    initial_url: String,
    surface_state: SharedSurfaceState,
    runtime: Option<GpuiRuntime>,
    webview: Option<WebViewHandle>,
    viewport: Option<SurfaceViewport>,
    focus_handle: FocusHandle,
    current_url: SharedString,
    load_status: SharedString,
    page_title: SharedString,
    last_status: SharedString,
    tick_task: Option<Task<()>>,
}

impl ServoGpuiExample {
    fn new(initial_url: String, cx: &mut Context<Self>) -> Self {
        let mut this = Self {
            initial_url,
            surface_state: new_surface_state(),
            runtime: None,
            webview: None,
            viewport: None,
            focus_handle: cx.focus_handle().tab_stop(true),
            current_url: "".into(),
            load_status: "not started".into(),
            page_title: "".into(),
            last_status: "Waiting for GPUI layout...".into(),
            tick_task: None,
        };
        this.start_tick(cx);
        this
    }

    fn start_tick(&mut self, cx: &mut Context<Self>) {
        self.tick_task = Some(cx.spawn(async move |this, cx| loop {
            cx.background_executor()
                .timer(Duration::from_millis(16))
                .await;
            if this
                .update(cx, |this, cx| {
                    this.pump();
                    cx.notify();
                })
                .is_err()
            {
                break;
            }
        }));
    }

    fn start_servo(&mut self, viewport: SurfaceViewport) -> Result<(), servokit::ServokitError> {
        let services = GpuiServoSurface::new(self.surface_state.clone());
        let options = SurfaceHostOptions::new(Arc::new(|| {}), Rc::new(MemoryClipboard::default()));
        let mut runtime = Runtime::new(SurfaceHost::new(services, options));
        let session = runtime.create_session();
        let webview = runtime.create_webview(session)?;

        runtime.load_url(webview, &self.initial_url)?;
        runtime.attach_surface_with_viewport(webview, HostSurface::new(SURFACE_ID), viewport)?;
        runtime.perform_updates(webview)?;

        self.webview = Some(webview);
        self.runtime = Some(runtime);
        self.drain_events();
        Ok(())
    }

    fn pump(&mut self) {
        if let Err(error) = self.sync_viewport() {
            self.last_status = format!("Servokit surface sync failed: {error}").into();
            return;
        }
        if let Some(error) = take_last_error(&self.surface_state) {
            self.last_status = format!("Servokit surface unavailable: {error}").into();
        }
        let (Some(runtime), Some(webview)) = (self.runtime.as_mut(), self.webview) else {
            return;
        };
        if let Err(error) = runtime.perform_updates(webview) {
            self.last_status = format!("Servokit update failed: {error}").into();
            return;
        }
        self.drain_events();
    }

    fn sync_viewport(&mut self) -> Result<(), servokit::ServokitError> {
        let Some(viewport) = viewport(&self.surface_state) else {
            return Ok(());
        };
        if self.runtime.is_none() {
            self.start_servo(viewport)?;
            self.last_status = format!("Loading {}", self.initial_url).into();
        } else if self.viewport != Some(viewport) {
            if let (Some(runtime), Some(webview)) = (self.runtime.as_mut(), self.webview) {
                runtime.update_surface_viewport(webview, viewport)?;
                self.drain_events();
            }
        }
        self.viewport = Some(viewport);
        Ok(())
    }

    fn dispatch_input(&mut self, input: HostInputEvent) {
        let label = format!("Input: {input:?}");
        let (Some(runtime), Some(webview)) = (self.runtime.as_mut(), self.webview) else {
            return;
        };
        match runtime.dispatch_input_event(webview, input) {
            Ok(()) => {
                self.last_status = label.into();
                self.drain_events();
            }
            Err(error) => self.last_status = format!("Servokit input failed: {error}").into(),
        }
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
                if event.managed_child_webview_id.is_none() {
                    if let HostEvent::NavigationRequested { navigation_id, .. } = &event.event {
                        navigation_requests.push((event.webview, navigation_id.clone()));
                    }
                }
                match &event.event {
                    HostEvent::UrlChanged { url } => self.current_url = url.clone().into(),
                    HostEvent::LoadStatusChanged { status } => {
                        self.load_status = status.as_str().into();
                    }
                    HostEvent::PageTitleChanged { title } => {
                        self.page_title = title.clone().unwrap_or_default().into();
                    }
                    HostEvent::Error { message, .. } => {
                        self.load_status = format!("error: {message}").into();
                    }
                    HostEvent::Crashed { reason, .. } => {
                        self.load_status = format!("crashed: {reason}").into();
                    }
                    _ => {}
                }
                self.last_status = format!("{:?}", event.event).into();
            }

            for (webview, navigation_id) in navigation_requests {
                let Some(runtime) = self.runtime.as_mut() else {
                    return;
                };
                if let Err(error) =
                    runtime.resolve_navigation_request(webview, &navigation_id, true)
                {
                    self.last_status =
                        format!("Servokit navigation request {navigation_id} failed: {error}")
                            .into();
                }
            }
        }
    }
}

impl Render for ServoGpuiExample {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let surface_state = self.surface_state.clone();
        let page_state = if self.page_title.is_empty() {
            format!("Load: {}", self.load_status)
        } else {
            format!("Load: {} · Title: {}", self.load_status, self.page_title)
        };

        div()
            .flex()
            .flex_col()
            .size_full()
            .bg(rgb(0x0f172a))
            .text_color(rgb(0xe2e8f0))
            .child(header(self.current_url.clone(), page_state))
            .child(self.browser_slot(cx, surface_state))
            .child(
                div()
                    .p_3()
                    .text_color(rgb(0x94a3b8))
                    .child(self.last_status.clone()),
            )
    }
}

impl ServoGpuiExample {
    fn browser_slot(
        &self,
        cx: &mut Context<Self>,
        surface_state: SharedSurfaceState,
    ) -> impl IntoElement {
        let down_state = surface_state.clone();
        let up_state = surface_state.clone();
        let move_state = surface_state.clone();
        let scroll_state = surface_state.clone();

        div()
            .flex()
            .flex_1()
            .overflow_hidden()
            .border_1()
            .border_color(rgb(0x334155))
            .bg(rgb(0x111827))
            .track_focus(&self.focus_handle)
            .on_children_prepainted(move |bounds, window, _cx| {
                if let Some(bounds) = bounds.first().copied() {
                    store_surface_state(&surface_state, bounds, window);
                }
            })
            .on_any_mouse_down(cx.listener(move |this, event, window, _cx| {
                this.focus_handle.focus(window);
                this.dispatch_input(HostInputEvent::Focus { is_focused: true });
                if let Some(input) = input::mouse_down(event, &down_state) {
                    this.dispatch_input(input);
                }
            }))
            .capture_any_mouse_up(cx.listener(move |this, event, _window, _cx| {
                if let Some(input) = input::mouse_up(event, &up_state) {
                    this.dispatch_input(input);
                }
            }))
            .on_mouse_move(cx.listener(move |this, event, _window, _cx| {
                if let Some(input) = input::mouse_move(event, &move_state) {
                    this.dispatch_input(input);
                }
            }))
            .on_scroll_wheel(cx.listener(move |this, event, _window, _cx| {
                if let Some(input) = input::scroll_wheel(event, &scroll_state) {
                    this.dispatch_input(input);
                }
            }))
            .on_mouse_down_out(cx.listener(|this, _event, _window, _cx| {
                this.dispatch_input(HostInputEvent::Focus { is_focused: false });
            }))
            .on_key_down(cx.listener(|this, event, _window, _cx| {
                if let Some(input) = input::key_down(event) {
                    this.dispatch_input(input);
                }
            }))
            .on_key_up(cx.listener(|this, event, _window, _cx| {
                if let Some(input) = input::key_up(event) {
                    this.dispatch_input(input);
                }
            }))
            .child(
                div()
                    .flex()
                    .size_full()
                    .items_center()
                    .justify_center()
                    .text_color(rgb(0x94a3b8))
                    .child("Servo native child view mounts here"),
            )
    }
}

fn header(current_url: SharedString, page_state: String) -> impl IntoElement {
    div()
        .flex()
        .flex_col()
        .p_4()
        .gap_2()
        .child(div().text_xl().child("Servokit inside a GPUI layout"))
        .child(
            div()
                .text_color(rgb(0x94a3b8))
                .child("GPUI owns this window and layout; Servokit owns the native child surface."),
        )
        .child(
            div()
                .text_color(rgb(0xbfdbfe))
                .child(format!("URL: {current_url}")),
        )
        .child(div().text_color(rgb(0x93c5fd)).child(page_state))
}

fn main() {
    let _ = ensure_default_rustls_crypto_provider();

    let initial_url = std::env::args()
        .nth(1)
        .unwrap_or_else(|| DEFAULT_INITIAL_URL.to_owned());

    Application::new().run(move |cx: &mut App| {
        let bounds = Bounds::centered(None, size(px(1100.0), px(760.0)), cx);
        cx.open_window(
            WindowOptions {
                window_bounds: Some(WindowBounds::Windowed(bounds)),
                ..Default::default()
            },
            |_, cx| cx.new(|cx| ServoGpuiExample::new(initial_url.clone(), cx)),
        )
        .unwrap();
    });
}
