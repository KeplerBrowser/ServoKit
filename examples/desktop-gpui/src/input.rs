use servokit::input::{
    HostInputEvent, KeyboardInputEvent, KeyboardInputKey, KeyboardInputState, KeyboardNamedKey,
    PointerButton, PointerButtonAction, PointerInputEvent, PointerScrollMode,
};
use zed_gpui::{
    KeyDownEvent, KeyUpEvent, Keystroke, MouseButton, MouseDownEvent, MouseMoveEvent, MouseUpEvent,
    ScrollDelta, ScrollWheelEvent,
};

use crate::appkit_surface::{local_device_point, SharedSurfaceState};

pub fn mouse_move(event: &MouseMoveEvent, state: &SharedSurfaceState) -> Option<HostInputEvent> {
    let (x, y) = local_device_point(state, event.position)?;
    Some(HostInputEvent::Pointer(PointerInputEvent::moved(x, y)))
}

pub fn mouse_down(event: &MouseDownEvent, state: &SharedSurfaceState) -> Option<HostInputEvent> {
    mouse_button(
        PointerButtonAction::Pressed,
        event.button,
        event.position,
        state,
    )
}

pub fn mouse_up(event: &MouseUpEvent, state: &SharedSurfaceState) -> Option<HostInputEvent> {
    mouse_button(
        PointerButtonAction::Released,
        event.button,
        event.position,
        state,
    )
}

pub fn scroll_wheel(
    event: &ScrollWheelEvent,
    state: &SharedSurfaceState,
) -> Option<HostInputEvent> {
    let (x, y) = local_device_point(state, event.position)?;
    let (delta_x, delta_y, mode) = match event.delta {
        ScrollDelta::Pixels(delta) => (
            delta.x.to_f64(),
            delta.y.to_f64(),
            PointerScrollMode::Pixels,
        ),
        ScrollDelta::Lines(delta) => (
            f64::from(delta.x),
            f64::from(delta.y),
            PointerScrollMode::Lines,
        ),
    };
    Some(HostInputEvent::Pointer(PointerInputEvent::wheel(
        delta_x, delta_y, mode, x, y,
    )))
}

pub fn key_down(event: &KeyDownEvent) -> Option<HostInputEvent> {
    keyboard_event(&event.keystroke, KeyboardInputState::Pressed, event.is_held)
}

pub fn key_up(event: &KeyUpEvent) -> Option<HostInputEvent> {
    keyboard_event(&event.keystroke, KeyboardInputState::Released, false)
}

fn mouse_button(
    action: PointerButtonAction,
    button: MouseButton,
    position: zed_gpui::Point<zed_gpui::Pixels>,
    state: &SharedSurfaceState,
) -> Option<HostInputEvent> {
    let (x, y) = local_device_point(state, position)?;
    Some(HostInputEvent::Pointer(PointerInputEvent::button(
        action,
        pointer_button(button),
        x,
        y,
    )))
}

fn pointer_button(button: MouseButton) -> PointerButton {
    match button {
        MouseButton::Left => PointerButton::Primary,
        MouseButton::Right => PointerButton::Secondary,
        MouseButton::Middle => PointerButton::Middle,
        MouseButton::Navigate(zed_gpui::NavigationDirection::Back) => PointerButton::Back,
        MouseButton::Navigate(zed_gpui::NavigationDirection::Forward) => PointerButton::Forward,
    }
}

fn keyboard_event(
    keystroke: &Keystroke,
    state: KeyboardInputState,
    repeat: bool,
) -> Option<HostInputEvent> {
    Some(HostInputEvent::Keyboard(KeyboardInputEvent {
        key: keyboard_key(keystroke)?,
        state,
        repeat,
        is_composing: false,
    }))
}

fn keyboard_key(keystroke: &Keystroke) -> Option<KeyboardInputKey> {
    let key = keystroke.key.as_str();
    let named = match key {
        "backspace" => Some(KeyboardNamedKey::Backspace),
        "delete" => Some(KeyboardNamedKey::Delete),
        "enter" | "return" => Some(KeyboardNamedKey::Enter),
        "tab" => Some(KeyboardNamedKey::Tab),
        "escape" | "esc" => Some(KeyboardNamedKey::Escape),
        "space" | " " => Some(KeyboardNamedKey::Space),
        "left" | "arrowleft" => Some(KeyboardNamedKey::ArrowLeft),
        "right" | "arrowright" => Some(KeyboardNamedKey::ArrowRight),
        "up" | "arrowup" => Some(KeyboardNamedKey::ArrowUp),
        "down" | "arrowdown" => Some(KeyboardNamedKey::ArrowDown),
        _ => None,
    };
    if let Some(named) = named {
        return Some(KeyboardInputKey::Named(named));
    }

    keystroke
        .key_char
        .as_ref()
        .filter(|value| !value.is_empty())
        .or_else(|| (!keystroke.key.is_empty()).then_some(&keystroke.key))
        .cloned()
        .map(KeyboardInputKey::Character)
}
