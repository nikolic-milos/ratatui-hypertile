use ratatui_hypertile::{
    HypertileEvent, KeyChord, KeyCode, Modifiers, MouseButton, MouseEvent, MouseEventKind,
};

pub fn keychord_from_crossterm(key: crossterm::event::KeyEvent) -> Option<KeyChord> {
    use crossterm::event::{KeyCode as CrosstermCode, KeyEventKind};

    if key.kind != KeyEventKind::Press {
        return None;
    }

    let code = match key.code {
        CrosstermCode::Char(c) => KeyCode::Char(c),
        CrosstermCode::Enter => KeyCode::Enter,
        CrosstermCode::Esc => KeyCode::Escape,
        CrosstermCode::Tab => KeyCode::Tab,
        CrosstermCode::BackTab => KeyCode::BackTab,
        CrosstermCode::Backspace => KeyCode::Backspace,
        CrosstermCode::Up => KeyCode::Up,
        CrosstermCode::Down => KeyCode::Down,
        CrosstermCode::Left => KeyCode::Left,
        CrosstermCode::Right => KeyCode::Right,
        CrosstermCode::Home => KeyCode::Home,
        CrosstermCode::End => KeyCode::End,
        CrosstermCode::PageUp => KeyCode::PageUp,
        CrosstermCode::PageDown => KeyCode::PageDown,
        CrosstermCode::Delete => KeyCode::Delete,
        CrosstermCode::Insert => KeyCode::Insert,
        CrosstermCode::F(n) => KeyCode::F(n),
        _ => return None,
    };

    Some(KeyChord {
        code,
        modifiers: modifiers_from_crossterm(key.modifiers),
    })
}

pub fn event_from_crossterm(key: crossterm::event::KeyEvent) -> Option<HypertileEvent> {
    keychord_from_crossterm(key).map(HypertileEvent::Key)
}

pub fn mouse_event_from_crossterm(mouse: crossterm::event::MouseEvent) -> MouseEvent {
    use crossterm::event::MouseEventKind as CrosstermMouseEventKind;

    let kind = match mouse.kind {
        CrosstermMouseEventKind::Down(button) => {
            MouseEventKind::Down(mouse_button_from_crossterm(button))
        }
        CrosstermMouseEventKind::Up(button) => {
            MouseEventKind::Up(mouse_button_from_crossterm(button))
        }
        CrosstermMouseEventKind::Drag(button) => {
            MouseEventKind::Drag(mouse_button_from_crossterm(button))
        }
        CrosstermMouseEventKind::Moved => MouseEventKind::Moved,
        CrosstermMouseEventKind::ScrollUp => MouseEventKind::ScrollUp,
        CrosstermMouseEventKind::ScrollDown => MouseEventKind::ScrollDown,
        CrosstermMouseEventKind::ScrollLeft => MouseEventKind::ScrollLeft,
        CrosstermMouseEventKind::ScrollRight => MouseEventKind::ScrollRight,
    };

    MouseEvent {
        kind,
        column: mouse.column,
        row: mouse.row,
        modifiers: modifiers_from_crossterm(mouse.modifiers),
    }
}

pub fn hypertile_event_from_crossterm(event: crossterm::event::Event) -> Option<HypertileEvent> {
    match event {
        crossterm::event::Event::Key(key) => event_from_crossterm(key),
        crossterm::event::Event::Mouse(mouse) => {
            Some(HypertileEvent::Mouse(mouse_event_from_crossterm(mouse)))
        }
        _ => None,
    }
}

fn modifiers_from_crossterm(modifiers: crossterm::event::KeyModifiers) -> Modifiers {
    let mut result = Modifiers::NONE;
    if modifiers.contains(crossterm::event::KeyModifiers::SHIFT) {
        result |= Modifiers::SHIFT;
    }
    if modifiers.contains(crossterm::event::KeyModifiers::CONTROL) {
        result |= Modifiers::CTRL;
    }
    if modifiers.contains(crossterm::event::KeyModifiers::ALT) {
        result |= Modifiers::ALT;
    }
    result
}

fn mouse_button_from_crossterm(button: crossterm::event::MouseButton) -> MouseButton {
    match button {
        crossterm::event::MouseButton::Left => MouseButton::Left,
        crossterm::event::MouseButton::Right => MouseButton::Right,
        crossterm::event::MouseButton::Middle => MouseButton::Middle,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crossterm::event::{
        Event as CrosstermEvent, KeyCode as CrosstermKeyCode, KeyEvent as CrosstermKeyEvent,
        KeyEventKind, KeyModifiers, MouseButton as CrosstermMouseButton,
        MouseEvent as CrosstermMouseEvent, MouseEventKind as CrosstermMouseEventKind,
    };

    #[test]
    fn key_adapter_ignores_non_press_events() {
        let press = CrosstermKeyEvent::new(CrosstermKeyCode::Char('s'), KeyModifiers::NONE);
        let release = CrosstermKeyEvent::new_with_kind(
            CrosstermKeyCode::Char('s'),
            KeyModifiers::NONE,
            KeyEventKind::Release,
        );

        assert_eq!(
            keychord_from_crossterm(press),
            Some(KeyChord {
                code: KeyCode::Char('s'),
                modifiers: Modifiers::NONE,
            })
        );
        assert_eq!(keychord_from_crossterm(release), None);
    }

    #[test]
    fn mouse_adapter_maps_kind_coordinates_and_modifiers() {
        let mouse = mouse_event_from_crossterm(CrosstermMouseEvent {
            kind: CrosstermMouseEventKind::Down(CrosstermMouseButton::Left),
            column: 7,
            row: 3,
            modifiers: KeyModifiers::CONTROL | KeyModifiers::SHIFT,
        });

        assert_eq!(mouse.kind, MouseEventKind::Down(MouseButton::Left));
        assert_eq!(mouse.column, 7);
        assert_eq!(mouse.row, 3);
        assert!(mouse.modifiers.contains(Modifiers::CTRL));
        assert!(mouse.modifiers.contains(Modifiers::SHIFT));
    }

    #[test]
    fn event_adapter_accepts_mouse_events() {
        let event = hypertile_event_from_crossterm(CrosstermEvent::Mouse(CrosstermMouseEvent {
            kind: CrosstermMouseEventKind::ScrollDown,
            column: 2,
            row: 9,
            modifiers: KeyModifiers::NONE,
        }));

        assert_eq!(
            event,
            Some(HypertileEvent::Mouse(MouseEvent::new(
                MouseEventKind::ScrollDown,
                2,
                9
            )))
        );
    }
}
