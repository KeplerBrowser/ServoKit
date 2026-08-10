use std::{cell::RefCell, rc::Rc};

use raw_window_handle::{HasDisplayHandle, HasWindowHandle};
use servokit::surface::{
    macos::{AppKitChildSurface, AppKitChildViewBounds},
    HostSurface, SurfaceDelegate, SurfaceError, SurfaceFrame, SurfaceTarget, SurfaceViewport,
};
use zed_gpui::{Bounds, Pixels, Point, Window};

pub type SharedSurfaceState = Rc<RefCell<GpuiSurfaceState>>;

pub fn new_surface_state() -> SharedSurfaceState {
    Rc::new(RefCell::new(GpuiSurfaceState::default()))
}

pub struct GpuiSurfaceState {
    child_surface: AppKitChildSurface,
    layout_bounds: Option<Bounds<Pixels>>,
    scale_factor: f32,
    last_error: Option<String>,
}

impl Default for GpuiSurfaceState {
    fn default() -> Self {
        Self {
            child_surface: AppKitChildSurface::new(),
            layout_bounds: None,
            scale_factor: 1.0,
            last_error: None,
        }
    }
}

#[derive(Clone)]
pub struct GpuiServoSurface {
    state: SharedSurfaceState,
}

impl GpuiServoSurface {
    pub fn new(state: SharedSurfaceState) -> Self {
        Self { state }
    }
}

impl SurfaceDelegate for GpuiServoSurface {
    type Frame<'a> = SurfaceFrame<'a>;

    fn render_target(
        &mut self,
        _surface: &HostSurface,
        _viewport: SurfaceViewport,
    ) -> Result<SurfaceTarget<'_>, SurfaceError> {
        let handles = {
            let state = self.state.borrow();
            state.child_surface.handles()?
        };
        Ok(handles.native_surface().into())
    }
}

pub fn viewport(state: &SharedSurfaceState) -> Option<SurfaceViewport> {
    state.borrow().child_surface.viewport()
}

pub fn take_last_error(state: &SharedSurfaceState) -> Option<String> {
    state.borrow_mut().last_error.take()
}

pub fn local_device_point(
    state: &SharedSurfaceState,
    position: Point<Pixels>,
) -> Option<(f32, f32)> {
    let state = state.borrow();
    let bounds = state.layout_bounds?;
    let scale = state.scale_factor;
    let x = ((position.x.to_f64() - bounds.origin.x.to_f64()) as f32 * scale).round();
    let y = ((position.y.to_f64() - bounds.origin.y.to_f64()) as f32 * scale).round();
    Some((x, y))
}

pub fn store_surface_state(state: &SharedSurfaceState, bounds: Bounds<Pixels>, window: &Window) {
    let mut state = state.borrow_mut();
    let display = match HasDisplayHandle::display_handle(window) {
        Ok(display) => display,
        Err(error) => return state.last_error = Some(format!("display handle: {error:?}")),
    };
    let parent = match HasWindowHandle::window_handle(window) {
        Ok(handle) => handle,
        Err(error) => return state.last_error = Some(format!("window handle: {error:?}")),
    };
    match state.child_surface.update(
        display,
        parent,
        AppKitChildViewBounds::new(
            bounds.origin.x.to_f64(),
            bounds.origin.y.to_f64(),
            bounds.size.width.to_f64(),
            bounds.size.height.to_f64(),
        ),
        window.scale_factor(),
    ) {
        Ok(_) => {}
        Err(error) => return state.last_error = Some(error.to_string()),
    }

    state.layout_bounds = Some(bounds);
    state.scale_factor = window.scale_factor();
    state.last_error = None;
}
