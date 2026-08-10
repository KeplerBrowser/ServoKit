use servokit::input::{
    HostInputEvent, KeyboardInputEvent, KeyboardInputKey, KeyboardInputState, KeyboardNamedKey,
    PointerButton, PointerButtonAction, PointerInputEvent, PointerScrollMode,
};
use servokit::surface::SurfaceSize;
use winit::{
    event::{ElementState, Ime, KeyEvent, MouseButton, MouseScrollDelta, WindowEvent},
    keyboard::{Key, NamedKey},
};

pub type CursorPoint = (f32, f32);

pub fn default_cursor_point(size: SurfaceSize) -> CursorPoint {
    (
        size.width.saturating_sub(1) as f32 / 2.0,
        size.height.saturating_sub(1) as f32 / 2.0,
    )
}

pub fn clamp_cursor_point(point: CursorPoint, size: SurfaceSize) -> CursorPoint {
    (
        point.0.clamp(0.0, size.width.saturating_sub(1) as f32),
        point.1.clamp(0.0, size.height.saturating_sub(1) as f32),
    )
}

pub fn to_servokit_event(
    event: &WindowEvent,
    cursor: &mut Option<CursorPoint>,
    viewport_size: SurfaceSize,
) -> Option<HostInputEvent> {
    match event {
        WindowEvent::Focused(is_focused) => Some(HostInputEvent::Focus {
            is_focused: *is_focused,
        }),
        WindowEvent::CursorMoved { position, .. } => {
            let point = clamp_cursor_point((position.x as f32, position.y as f32), viewport_size);
            *cursor = Some(point);
            Some(HostInputEvent::Pointer(PointerInputEvent::moved(
                point.0, point.1,
            )))
        }
        WindowEvent::CursorLeft { .. } => {
            *cursor = None;
            Some(HostInputEvent::Pointer(PointerInputEvent::left_viewport()))
        }
        WindowEvent::MouseInput { state, button, .. } => {
            let (x, y) = clamp_cursor_point((*cursor)?, viewport_size);
            *cursor = Some((x, y));
            Some(HostInputEvent::Pointer(PointerInputEvent::button(
                pointer_action(*state),
                pointer_button(*button),
                x,
                y,
            )))
        }
        WindowEvent::MouseWheel { delta, .. } => {
            let point = (*cursor).unwrap_or_else(|| default_cursor_point(viewport_size));
            let (x, y) = clamp_cursor_point(point, viewport_size);
            if cursor.is_some() {
                *cursor = Some((x, y));
            }
            let (delta_x, delta_y, mode) = wheel_delta(*delta);
            Some(HostInputEvent::Pointer(PointerInputEvent::wheel(
                delta_x, delta_y, mode, x, y,
            )))
        }
        WindowEvent::KeyboardInput {
            event,
            is_synthetic: false,
            ..
        } => keyboard_input(event).map(HostInputEvent::Keyboard),
        WindowEvent::Ime(Ime::Commit(text)) if !text.is_empty() => {
            Some(HostInputEvent::ImeCommit { text: text.clone() })
        }
        _ => None,
    }
}

fn pointer_action(state: ElementState) -> PointerButtonAction {
    match state {
        ElementState::Pressed => PointerButtonAction::Pressed,
        ElementState::Released => PointerButtonAction::Released,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use servokit::input::PointerInputEvent;
    use winit::event::{DeviceId, ElementState, MouseButton, MouseScrollDelta, WindowEvent};

    const VIEWPORT: SurfaceSize = SurfaceSize {
        width: 100,
        height: 50,
    };

    #[test]
    fn mouse_button_before_first_cursor_move_is_dropped() {
        let mut cursor = None;
        let event = WindowEvent::MouseInput {
            device_id: DeviceId::dummy(),
            state: ElementState::Pressed,
            button: MouseButton::Left,
        };

        assert_eq!(to_servokit_event(&event, &mut cursor, VIEWPORT), None);
        assert_eq!(cursor, None);
    }

    #[test]
    fn mouse_wheel_before_first_cursor_move_uses_bounded_default_point() {
        let mut cursor = None;
        let event = WindowEvent::MouseWheel {
            device_id: DeviceId::dummy(),
            delta: MouseScrollDelta::LineDelta(0.0, 1.0),
            phase: winit::event::TouchPhase::Moved,
        };

        assert_eq!(
            to_servokit_event(&event, &mut cursor, VIEWPORT),
            Some(HostInputEvent::Pointer(PointerInputEvent::wheel(
                0.0,
                1.0,
                PointerScrollMode::Lines,
                49.5,
                24.5,
            )))
        );
        assert_eq!(cursor, None);
    }

    #[test]
    fn mouse_button_clamps_stale_real_cursor_to_viewport() {
        let mut cursor = Some((200.0, -20.0));
        let event = WindowEvent::MouseInput {
            device_id: DeviceId::dummy(),
            state: ElementState::Pressed,
            button: MouseButton::Left,
        };

        assert_eq!(
            to_servokit_event(&event, &mut cursor, VIEWPORT),
            Some(HostInputEvent::Pointer(PointerInputEvent::button(
                PointerButtonAction::Pressed,
                PointerButton::Primary,
                99.0,
                0.0,
            )))
        );
        assert_eq!(cursor, Some((99.0, 0.0)));
    }

    #[test]
    fn cursor_left_clears_real_cursor() {
        let mut cursor = Some((10.0, 20.0));
        let event = WindowEvent::CursorLeft {
            device_id: DeviceId::dummy(),
        };

        assert_eq!(
            to_servokit_event(&event, &mut cursor, VIEWPORT),
            Some(HostInputEvent::Pointer(PointerInputEvent::left_viewport()))
        );
        assert_eq!(cursor, None);
    }
}

fn pointer_button(button: MouseButton) -> PointerButton {
    match button {
        MouseButton::Left => PointerButton::Primary,
        MouseButton::Right => PointerButton::Secondary,
        MouseButton::Middle => PointerButton::Middle,
        MouseButton::Back => PointerButton::Back,
        MouseButton::Forward => PointerButton::Forward,
        MouseButton::Other(value) => PointerButton::Other(value),
    }
}

fn wheel_delta(delta: MouseScrollDelta) -> (f64, f64, PointerScrollMode) {
    match delta {
        MouseScrollDelta::LineDelta(x, y) => (f64::from(x), f64::from(y), PointerScrollMode::Lines),
        MouseScrollDelta::PixelDelta(delta) => (delta.x, delta.y, PointerScrollMode::Pixels),
    }
}

fn keyboard_input(event: &KeyEvent) -> Option<KeyboardInputEvent> {
    let key = match &event.logical_key {
        Key::Character(value) => KeyboardInputKey::Character(
            event
                .text
                .as_ref()
                .filter(|text| !text.is_empty())
                .map(ToString::to_string)
                .unwrap_or_else(|| value.to_string()),
        ),
        Key::Named(named) => KeyboardInputKey::Named(named_key(*named)?),
        Key::Unidentified(_) | Key::Dead(_) => return None,
    };
    Some(KeyboardInputEvent {
        key,
        state: match event.state {
            ElementState::Pressed => KeyboardInputState::Pressed,
            ElementState::Released => KeyboardInputState::Released,
        },
        repeat: event.repeat,
        is_composing: false,
    })
}

fn named_key(key: NamedKey) -> Option<KeyboardNamedKey> {
    match key {
        NamedKey::Backspace => Some(KeyboardNamedKey::Backspace),
        NamedKey::Delete => Some(KeyboardNamedKey::Delete),
        NamedKey::Enter => Some(KeyboardNamedKey::Enter),
        NamedKey::Tab => Some(KeyboardNamedKey::Tab),
        NamedKey::Escape => Some(KeyboardNamedKey::Escape),
        NamedKey::Space => Some(KeyboardNamedKey::Space),
        NamedKey::ArrowLeft => Some(KeyboardNamedKey::ArrowLeft),
        NamedKey::ArrowRight => Some(KeyboardNamedKey::ArrowRight),
        NamedKey::ArrowUp => Some(KeyboardNamedKey::ArrowUp),
        NamedKey::ArrowDown => Some(KeyboardNamedKey::ArrowDown),
        _ => None,
    }
}
