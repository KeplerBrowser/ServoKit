use std::{
    cell::RefCell,
    collections::HashMap,
    ptr::NonNull,
    rc::Rc,
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc,
    },
    time::{Duration, Instant},
};

use cocoa::{
    appkit::{
        NSApp, NSApplication, NSApplicationActivationPolicyRegular, NSBackingStoreBuffered,
        NSEventMask, NSView, NSWindow, NSWindowStyleMask,
    },
    base::{id, nil, NO, YES},
    foundation::{
        NSAutoreleasePool, NSDate, NSDefaultRunLoopMode, NSPoint, NSRect, NSSize, NSString,
    },
};
use raw_window_handle::{
    AppKitDisplayHandle, AppKitWindowHandle, DisplayHandle, RawDisplayHandle, RawWindowHandle,
    WindowHandle,
};
use servokit::{
    events::{HostEvent, LoadStatusKind},
    input::{
        HostInputEvent, KeyboardInputEvent, KeyboardInputKey, KeyboardInputState, PointerButton,
        PointerButtonAction, PointerInputEvent,
    },
    runtime::{Runtime, SessionHandle},
    surface::{
        HostSurface, MemoryClipboard, NativeSurface, SurfaceDelegate, SurfaceError, SurfaceFrame,
        SurfaceHost, SurfaceHostOptions, SurfaceSize, SurfaceTarget, SurfaceViewport,
    },
    webview::WebViewHandle,
};

type NativeRuntime = Runtime<SurfaceHost<Surfaces>>;

#[derive(Clone, Default)]
struct Surfaces(Rc<RefCell<HashMap<String, id>>>);

impl SurfaceDelegate for Surfaces {
    type Frame<'a> = SurfaceFrame<'a>;
    fn render_target(
        &mut self,
        surface: &HostSurface,
        _: SurfaceViewport,
    ) -> Result<SurfaceTarget<'_>, SurfaceError> {
        let child = *self
            .0
            .borrow()
            .get(surface.id())
            .ok_or_else(|| SurfaceError::new("unknown native child"))?;
        // SAFETY: the example's window owns this child until close_view first destroys
        // the Servo view. All calls occur on the AppKit main thread.
        unsafe {
            Ok(NativeSurface::new(
                DisplayHandle::borrow_raw(RawDisplayHandle::AppKit(AppKitDisplayHandle::new())),
                WindowHandle::borrow_raw(RawWindowHandle::AppKit(AppKitWindowHandle::new(
                    NonNull::new(child.cast()).unwrap(),
                ))),
            )
            .into())
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct Page {
    surface: HostSurface,
    title: String,
    complete: bool,
    url: String,
    history: Option<HostEvent>,
    focused: Option<bool>,
}

struct Proof {
    // Explicit close_view/shutdown releases rendering before removing the children.
    runtime: NativeRuntime,
    surfaces: Surfaces,
    pages: HashMap<WebViewHandle, Page>,
    session: SessionHandle,
    window: id,
    evaluations: HashMap<(WebViewHandle, String), String>,
    wake: Arc<AtomicBool>,
}

impl Proof {
    fn new(window: id) -> Self {
        let surfaces = Surfaces::default();
        let wake = Arc::new(AtomicBool::new(false));
        let signal = wake.clone();
        let mut runtime = Runtime::new(SurfaceHost::new(
            surfaces.clone(),
            SurfaceHostOptions::new(
                Arc::new(move || {
                    signal.store(true, Ordering::Release);
                }),
                Rc::new(MemoryClipboard::default()),
            ),
        ));
        let session = runtime.create_session();
        Self {
            runtime,
            surfaces,
            pages: HashMap::new(),
            session,
            window,
            evaluations: HashMap::new(),
            wake,
        }
    }

    fn create_view(&mut self, title: &str, x: f64) -> WebViewHandle {
        let view = self.runtime.create_webview(self.session).unwrap();
        let surface = HostSurface::new(format!("page-{}", view.raw()));
        unsafe {
            let pool = NSAutoreleasePool::new(nil);
            let child = NSView::alloc(nil)
                .initWithFrame_(NSRect::new(
                    NSPoint::new(x, 20.0),
                    NSSize::new(440.0, 520.0),
                ))
                .autorelease();
            assert!(!child.is_null());
            child.setWantsLayer(YES);
            child.setWantsBestResolutionOpenGLSurface_(YES);
            self.window.contentView().addSubview_(child);
            self.surfaces
                .0
                .borrow_mut()
                .insert(surface.id().into(), child);
            pool.drain(); // The parent retains the child until close_view removes it.
        }
        self.pages.insert(
            view,
            Page {
                surface: surface.clone(),
                title: String::new(),
                complete: false,
                url: String::new(),
                history: None,
                focused: None,
            },
        );
        self.runtime.load_url(view, page_url(title)).unwrap();
        let scale = unsafe { self.window.backingScaleFactor() as f32 };
        self.runtime
            .attach_surface_with_viewport(
                view,
                surface,
                SurfaceViewport::new(
                    SurfaceSize::new((440.0 * scale) as u32, (520.0 * scale) as u32),
                    scale,
                ),
            )
            .unwrap();
        view
    }

    fn close_view(&mut self, view: WebViewHandle) {
        self.runtime.destroy_webview(view).unwrap();
        let page = self.pages.remove(&view).unwrap();
        let child = self
            .surfaces
            .0
            .borrow_mut()
            .remove(page.surface.id())
            .unwrap();
        unsafe {
            child.removeFromSuperview();
        }
        self.evaluations.retain(|(owner, _), _| *owner != view);
        assert!(self.runtime.reload(view).is_err());
        println!("closed view={}", view.raw());
    }

    fn pump(&mut self) {
        unsafe {
            let pool = NSAutoreleasePool::new(nil);
            loop {
                let event = NSApp().nextEventMatchingMask_untilDate_inMode_dequeue_(
                    NSEventMask::NSAnyEventMask.bits(),
                    NSDate::distantPast(nil),
                    NSDefaultRunLoopMode,
                    YES,
                );
                if event == nil {
                    break;
                }
                NSApp().sendEvent_(event);
            }
            pool.drain();
        }
        self.runtime.perform_all_updates().unwrap();
        let mut navigation = Vec::new();
        for event in self.runtime.drain_events() {
            let page = self
                .pages
                .get_mut(&event.webview)
                .expect("event must target a live view");
            match event.event {
                HostEvent::NavigationRequested { navigation_id, .. } => {
                    navigation.push((event.webview, navigation_id))
                }
                HostEvent::PageTitleChanged { title } => {
                    page.title = title.unwrap_or_default();
                    println!("title view={} title={}", event.webview.raw(), page.title);
                }
                HostEvent::LoadStatusChanged { status } => {
                    page.complete = status == LoadStatusKind::Complete
                }
                HostEvent::UrlChanged { url } => page.url = url,
                history @ HostEvent::HistoryChanged { .. } => page.history = Some(history),
                HostEvent::FocusChanged { is_focused } => page.focused = Some(is_focused),
                HostEvent::JavaScriptEvaluationResult {
                    evaluation_id,
                    ok,
                    value_json,
                    error_type,
                } => {
                    assert!(ok, "evaluation failed: {error_type:?}");
                    self.evaluations
                        .insert((event.webview, evaluation_id), value_json.unwrap());
                }
                HostEvent::Error { message, .. } => {
                    panic!("view {}: {message}", event.webview.raw())
                }
                HostEvent::Crashed { reason, .. } => {
                    panic!("view {} crashed: {reason}", event.webview.raw())
                }
                _ => {}
            }
        }
        for (view, request) in navigation {
            self.runtime
                .resolve_navigation_request(view, &request, true)
                .unwrap();
        }
    }

    fn wait(&mut self, done: impl Fn(&Self) -> bool) {
        let deadline = Instant::now() + Duration::from_secs(30);
        loop {
            self.pump();
            if done(self) {
                return;
            }
            assert!(Instant::now() < deadline, "native proof timed out");
            std::thread::sleep(Duration::from_millis(5));
        }
    }

    fn loaded(&mut self, view: WebViewHandle, title: &str) {
        self.wait(|this| {
            this.pages.get(&view).is_some_and(|page| {
                page.complete
                    && page.title == title
                    && !page.url.is_empty()
                    && page.history.is_some()
            })
        });
    }

    fn assert_history(&self, view: WebViewHandle, url: &str) {
        let page = &self.pages[&view];
        assert_eq!(page.url, url, "URL event routed to the wrong view");
        let Some(HostEvent::HistoryChanged {
            entries, current, ..
        }) = &page.history
        else {
            panic!("missing history event for view {}", view.raw());
        };
        assert_eq!(
            entries[*current], url,
            "history event routed to the wrong view"
        );
    }

    fn assert_js(&mut self, view: WebViewHandle, script: &str) {
        // Input/layout delivery and JS evaluation travel through different Servo queues.
        // Poll the observable page state rather than assuming the next evaluation fences input.
        let deadline = Instant::now() + Duration::from_secs(10);
        loop {
            let key = (view, "proof".to_owned());
            self.evaluations.remove(&key);
            self.runtime
                .evaluate_javascript(view, "proof", script)
                .unwrap();
            self.wait(|this| this.evaluations.contains_key(&key));
            let result = self.evaluations.remove(&key).unwrap();
            if result == r#"{"type":"boolean","value":true}"# {
                return;
            }
            assert!(
                Instant::now() < deadline,
                "view {}: {script} returned {result}",
                view.raw()
            );
        }
    }

    fn resize(&mut self, view: WebViewHandle, width: f64) {
        let child = self.surfaces.0.borrow()[self.pages[&view].surface.id()];
        unsafe {
            child.setFrameSize(NSSize::new(width, 520.0));
        }
        let scale = unsafe { self.window.backingScaleFactor() as f32 };
        self.runtime
            .update_surface_viewport(
                view,
                SurfaceViewport::new(
                    SurfaceSize::new((width * scale as f64) as u32, (520.0 * scale as f64) as u32),
                    scale,
                ),
            )
            .unwrap();
    }

    fn smoke(&mut self, first: WebViewHandle, second: WebViewHandle) {
        self.loaded(first, "First");
        self.loaded(second, "Second");
        self.assert_js(first, "document.title === 'First'");
        self.assert_js(second, "document.title === 'Second'");
        let first_url = self.pages[&first].url.clone();
        let second_url = self.pages[&second].url.clone();
        assert_ne!(first_url, second_url);
        self.assert_history(first, &first_url);
        self.assert_history(second, &second_url);
        self.runtime.focus(first).unwrap();
        self.wait(|this| this.pages[&first].focused == Some(true));
        let second_state = self.pages[&second].clone();
        self.runtime.blur(first).unwrap();
        self.wait(|this| this.pages[&first].focused == Some(false));
        assert_eq!(self.pages[&second], second_state);
        self.runtime.focus(first).unwrap();
        self.wait(|this| this.pages[&first].focused == Some(true));
        self.runtime
            .dispatch_input_event(
                first,
                HostInputEvent::Pointer(PointerInputEvent::moved(40.0, 40.0)),
            )
            .unwrap();
        self.runtime
            .dispatch_input_event(
                first,
                HostInputEvent::Pointer(PointerInputEvent::button(
                    PointerButtonAction::Pressed,
                    PointerButton::Primary,
                    40.0,
                    40.0,
                )),
            )
            .unwrap();
        self.runtime
            .dispatch_input_event(
                first,
                HostInputEvent::Pointer(PointerInputEvent::button(
                    PointerButtonAction::Released,
                    PointerButton::Primary,
                    40.0,
                    40.0,
                )),
            )
            .unwrap();
        self.runtime
            .dispatch_input_event(
                first,
                HostInputEvent::Keyboard(KeyboardInputEvent::new(
                    KeyboardInputKey::Character("x".into()),
                    KeyboardInputState::Pressed,
                )),
            )
            .unwrap();
        self.assert_js(first, "window.clicks === 1 && window.keys === 'x'");
        self.assert_js(second, "window.clicks === 0 && window.keys === ''");
        assert_eq!(self.pages[&second], second_state);
        self.resize(first, 360.0);
        self.assert_js(first, "innerWidth === 360");
        self.assert_js(second, "innerWidth === 440");
        self.runtime.load_url(first, page_url("Navigated")).unwrap();
        self.loaded(first, "Navigated");
        let navigated_url = self.pages[&first].url.clone();
        assert_ne!(navigated_url, first_url);
        self.assert_history(first, &navigated_url);
        assert!(matches!(
            self.pages[&first].history,
            Some(HostEvent::HistoryChanged {
                can_go_back: true,
                can_go_forward: false,
                ..
            })
        ));
        assert_eq!(self.pages[&second], second_state);
        self.runtime.go_back(first).unwrap();
        self.loaded(first, "First");
        self.assert_history(first, &first_url);
        assert!(matches!(
            self.pages[&first].history,
            Some(HostEvent::HistoryChanged {
                can_go_forward: true,
                ..
            })
        ));
        self.assert_js(second, "document.title === 'Second'");
        assert_eq!(self.pages[&second], second_state);
        self.runtime.go_forward(first).unwrap();
        self.loaded(first, "Navigated");
        self.assert_history(first, &navigated_url);
        assert_eq!(self.pages[&second], second_state);
        self.assert_js(second, "(window.survived = 1, scrollTo(0, 120), true)");
        self.assert_js(second, "window.survived === 1 && scrollY === 120");
        self.close_view(first);
        self.assert_js(
            second,
            "document.title === 'Second' && window.survived === 1 && scrollY === 120",
        );
        assert_eq!(self.pages[&second], second_state);
        self.wake.store(false, Ordering::Release);
        self.assert_js(
            second,
            "(setTimeout(() => document.title = 'Awake', 20), true)",
        );
        self.wait(|this| this.pages[&second].title == "Awake");
        assert!(
            self.wake.load(Ordering::Acquire),
            "closing first view disconnected engine wakeups"
        );
        let replacement = self.create_view("Replacement", 20.0);
        self.loaded(replacement, "Replacement");
        let survivor_state = self.pages[&second].clone();
        self.close_view(replacement); // Now close the later-created sibling first.
        self.assert_js(
            second,
            "document.title === 'Awake' && window.survived === 1 && scrollY === 120",
        );
        assert_eq!(self.pages[&second], survivor_state);
        self.close_view(second);
        self.runtime.perform_all_updates().unwrap();
        let after_empty = self.create_view("After empty", 20.0);
        self.loaded(after_empty, "After empty");
        self.close_view(after_empty);
    }
}

fn page_url(title: &str) -> String {
    format!(
        "data:text/html,<title>{title}</title><body style='margin:0;background:aliceblue;font:24px sans-serif;height:1800px'><button style='width:180px;height:80px'>Click {title}</button><h1>{title}</h1><input placeholder='Each page is independent'><script>window.clicks=0;window.keys='';document.addEventListener('click',()=>window.clicks++);document.addEventListener('keydown',e=>window.keys+=e.key);</script>"
    )
}

pub fn run() {
    // AppKit and Servo both stay on the process main thread.
    unsafe {
        let pool = NSAutoreleasePool::new(nil);
        let app = NSApp();
        app.setActivationPolicy_(NSApplicationActivationPolicyRegular);
        let window = NSWindow::alloc(nil)
            .initWithContentRect_styleMask_backing_defer_(
                NSRect::new(NSPoint::new(100.0, 100.0), NSSize::new(940.0, 560.0)),
                NSWindowStyleMask::NSTitledWindowMask | NSWindowStyleMask::NSClosableWindowMask,
                NSBackingStoreBuffered,
                NO,
            )
            .autorelease();
        window.setReleasedWhenClosed_(NO);
        window.setTitle_(
            NSString::alloc(nil)
                .init_str("ServoKit — two independent native views")
                .autorelease(),
        );
        window.makeKeyAndOrderFront_(nil);
        app.finishLaunching();
        app.activateIgnoringOtherApps_(YES);
        let mut proof = Proof::new(window);
        let first = proof.create_view("First", 20.0);
        let second = proof.create_view("Second", 480.0);
        let smoke = std::env::args().any(|arg| arg == "--smoke");
        if smoke {
            proof.smoke(first, second);
        } else {
            while window.isVisible() == YES {
                proof.pump();
                std::thread::sleep(Duration::from_millis(16));
            }
            for view in proof.pages.keys().copied().collect::<Vec<_>>() {
                proof.close_view(view);
            }
        }
        proof.runtime.shutdown().unwrap();
        window.close();
        pool.drain();
        if smoke {
            println!("multiple-native-views result=pass");
        }
    }
}
