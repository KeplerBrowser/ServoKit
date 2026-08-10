use servokit_embedder::{
    ContextMenuAction, HostEvent, ImeCompositionState, KeyboardKey, SurfaceSize, TouchEventKind,
};

pub type NativeWindowHandle = *mut std::ffi::c_void;

pub(crate) trait AndroidRenderBackend {
    fn attach_surface(
        &mut self,
        native_window: NativeWindowHandle,
        size: SurfaceSize,
        density: f32,
    ) -> Result<Vec<HostEvent>, String>;

    fn resize_surface(&mut self, size: SurfaceSize, density: f32)
        -> Result<Vec<HostEvent>, String>;

    fn detach_surface(&mut self) -> Result<Vec<HostEvent>, String>;

    fn load_url(&mut self, url: &str) -> Result<Vec<HostEvent>, String>;

    fn reload(&mut self) -> Result<Vec<HostEvent>, String>;

    fn go_back(&mut self) -> Result<Vec<HostEvent>, String>;

    fn go_forward(&mut self) -> Result<Vec<HostEvent>, String>;

    fn focus(&mut self) -> Result<Vec<HostEvent>, String>;

    fn blur(&mut self) -> Result<Vec<HostEvent>, String>;

    fn evaluate_javascript(
        &mut self,
        _evaluation_id: &str,
        _script: &str,
    ) -> Result<Vec<HostEvent>, String> {
        Ok(Vec::new())
    }

    fn dispatch_touch_event(
        &mut self,
        event_kind: TouchEventKind,
        touch_id: i32,
        x: f32,
        y: f32,
    ) -> Result<Vec<HostEvent>, String>;

    fn trigger_context_menu(&mut self, _x: f32, _y: f32) -> Result<Vec<HostEvent>, String> {
        Ok(Vec::new())
    }

    fn dispatch_ime_composition(
        &mut self,
        _state: ImeCompositionState,
        _text: &str,
    ) -> Result<Vec<HostEvent>, String> {
        Ok(Vec::new())
    }

    fn dismiss_input_method(&mut self) -> Result<Vec<HostEvent>, String> {
        Ok(Vec::new())
    }

    fn dispatch_keyboard_key(&mut self, _key: KeyboardKey) -> Result<Vec<HostEvent>, String> {
        Ok(Vec::new())
    }

    fn resolve_simple_dialog(
        &mut self,
        _dialog_id: &str,
        _confirmed: bool,
        _prompt_value: Option<&str>,
    ) -> Result<Vec<HostEvent>, String> {
        Ok(Vec::new())
    }

    fn resolve_select_element(
        &mut self,
        _select_element_id: &str,
        _selected_options: Vec<usize>,
    ) -> Result<Vec<HostEvent>, String> {
        Ok(Vec::new())
    }

    fn resolve_file_picker(
        &mut self,
        _file_picker_id: &str,
        _selected_paths: Vec<String>,
    ) -> Result<Vec<HostEvent>, String> {
        Ok(Vec::new())
    }

    fn dismiss_file_picker(&mut self, _file_picker_id: &str) -> Result<Vec<HostEvent>, String> {
        Ok(Vec::new())
    }

    fn has_pending_permission_request(&self) -> bool {
        false
    }

    fn pending_permission_request(&self) -> Option<(String, String)> {
        None
    }

    fn resolve_permission(&mut self, _allow: bool) -> Result<Vec<HostEvent>, String> {
        Ok(Vec::new())
    }

    fn resolve_navigation_request(
        &mut self,
        _navigation_id: &str,
        _allow: bool,
    ) -> Result<Vec<HostEvent>, String> {
        Ok(Vec::new())
    }

    fn resolve_context_menu(
        &mut self,
        _context_menu_id: &str,
        _action: ContextMenuAction,
    ) -> Result<Vec<HostEvent>, String> {
        Ok(Vec::new())
    }

    fn dismiss_context_menu(&mut self, _context_menu_id: &str) -> Result<Vec<HostEvent>, String> {
        Ok(Vec::new())
    }

    fn perform_updates(&mut self) -> Result<Vec<HostEvent>, String>;

    fn shutdown(&mut self);
}

#[cfg(test)]
mod tests {
    use super::*;

    struct MinimalAndroidRenderBackend;

    impl AndroidRenderBackend for MinimalAndroidRenderBackend {
        fn attach_surface(
            &mut self,
            _native_window: NativeWindowHandle,
            _size: SurfaceSize,
            _density: f32,
        ) -> Result<Vec<HostEvent>, String> {
            Ok(Vec::new())
        }

        fn resize_surface(
            &mut self,
            _size: SurfaceSize,
            _density: f32,
        ) -> Result<Vec<HostEvent>, String> {
            Ok(Vec::new())
        }

        fn detach_surface(&mut self) -> Result<Vec<HostEvent>, String> {
            Ok(Vec::new())
        }

        fn load_url(&mut self, _url: &str) -> Result<Vec<HostEvent>, String> {
            Ok(Vec::new())
        }

        fn reload(&mut self) -> Result<Vec<HostEvent>, String> {
            Ok(Vec::new())
        }

        fn go_back(&mut self) -> Result<Vec<HostEvent>, String> {
            Ok(Vec::new())
        }

        fn go_forward(&mut self) -> Result<Vec<HostEvent>, String> {
            Ok(Vec::new())
        }

        fn focus(&mut self) -> Result<Vec<HostEvent>, String> {
            Ok(Vec::new())
        }

        fn blur(&mut self) -> Result<Vec<HostEvent>, String> {
            Ok(Vec::new())
        }

        fn dispatch_touch_event(
            &mut self,
            _event_kind: TouchEventKind,
            _touch_id: i32,
            _x: f32,
            _y: f32,
        ) -> Result<Vec<HostEvent>, String> {
            Ok(Vec::new())
        }

        fn perform_updates(&mut self) -> Result<Vec<HostEvent>, String> {
            Ok(Vec::new())
        }

        fn shutdown(&mut self) {}
    }

    #[test]
    fn optional_android_host_capabilities_default_to_noop() {
        let mut backend = MinimalAndroidRenderBackend;

        assert_eq!(backend.trigger_context_menu(12.0, 24.0), Ok(Vec::new()));
        assert_eq!(
            backend.dispatch_ime_composition(ImeCompositionState::Update, "servo"),
            Ok(Vec::new())
        );
        assert_eq!(backend.dismiss_input_method(), Ok(Vec::new()));
        assert_eq!(
            backend.dispatch_keyboard_key(KeyboardKey::Enter),
            Ok(Vec::new())
        );
        assert_eq!(
            backend.resolve_simple_dialog("dialog-1", true, Some("servo")),
            Ok(Vec::new())
        );
        assert_eq!(
            backend.resolve_select_element("select-1", vec![1, 4]),
            Ok(Vec::new())
        );
        assert_eq!(
            backend.resolve_file_picker("file-picker-1", vec!["/tmp/file.txt".to_owned()]),
            Ok(Vec::new())
        );
        assert_eq!(backend.dismiss_file_picker("file-picker-1"), Ok(Vec::new()));
        assert!(!backend.has_pending_permission_request());
        assert_eq!(backend.pending_permission_request(), None);
        assert_eq!(backend.resolve_permission(true), Ok(Vec::new()));
        assert_eq!(
            backend.resolve_navigation_request("navigation-1", false),
            Ok(Vec::new())
        );
        assert_eq!(
            backend.resolve_context_menu("context-menu-1", ContextMenuAction::CopyLink),
            Ok(Vec::new())
        );
        assert_eq!(
            backend.dismiss_context_menu("context-menu-1"),
            Ok(Vec::new())
        );
    }
}
