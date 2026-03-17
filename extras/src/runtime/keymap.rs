use crate::runtime::HypertileRuntime;
use ratatui::layout::Direction;
use ratatui_hypertile::{HypertileAction, KeyChord, KeyCode, Modifiers, Towards};

#[derive(Debug, Clone, PartialEq)]
pub(super) enum RuntimeAction {
    Core(HypertileAction),
    SplitDefault(Direction),
    OpenPalette,
    InteractFocused,
    EnterPluginInput,
}

/// Movement key preset for layout mode.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MoveBindings {
    /// `hjkl` (uppercase or with Shift) to move panes.
    Vim,
    /// Shift+Arrow keys to move panes.
    ShiftArrows,
    /// Both vim keys and Shift+Arrow keys.
    VimAndShiftArrows,
}

impl MoveBindings {
    fn includes_vim(self) -> bool {
        matches!(self, Self::Vim | Self::VimAndShiftArrows)
    }

    fn includes_shift_arrows(self) -> bool {
        matches!(self, Self::ShiftArrows | Self::VimAndShiftArrows)
    }
}

impl HypertileRuntime {
    pub(super) fn default_layout_action(&self, chord: KeyChord) -> Option<RuntimeAction> {
        const SHIFT_ARROW_MOVES: [(KeyCode, Direction, Towards); 4] = [
            (KeyCode::Left, Direction::Horizontal, Towards::Start),
            (KeyCode::Right, Direction::Horizontal, Towards::End),
            (KeyCode::Down, Direction::Vertical, Towards::End),
            (KeyCode::Up, Direction::Vertical, Towards::Start),
        ];

        if self.move_bindings.includes_shift_arrows() && chord.modifiers == Modifiers::SHIFT {
            for &(code, direction, towards) in &SHIFT_ARROW_MOVES {
                if chord.code == code {
                    return Some(RuntimeAction::Core(HypertileAction::MoveFocused {
                        direction,
                        towards,
                        scope: self.default_move_scope,
                    }));
                }
            }
        }

        const VIM_MOVES: [(char, Direction, Towards); 4] = [
            ('h', Direction::Horizontal, Towards::Start),
            ('l', Direction::Horizontal, Towards::End),
            ('j', Direction::Vertical, Towards::End),
            ('k', Direction::Vertical, Towards::Start),
        ];

        if self.move_bindings.includes_vim() {
            for &(ch, direction, towards) in &VIM_MOVES {
                let upper = ch.to_ascii_uppercase();
                let matches = match (chord.code, chord.modifiers) {
                    (KeyCode::Char(c), Modifiers::SHIFT) if c == upper || c == ch => true,
                    (KeyCode::Char(c), Modifiers::NONE) if c == upper => true,
                    _ => false,
                };
                if matches {
                    return Some(RuntimeAction::Core(HypertileAction::MoveFocused {
                        direction,
                        towards,
                        scope: self.default_move_scope,
                    }));
                }
            }
        }

        if !chord.modifiers.is_empty() {
            return None;
        }

        match chord.code {
            KeyCode::Tab => Some(RuntimeAction::Core(HypertileAction::FocusNext)),
            KeyCode::BackTab => Some(RuntimeAction::Core(HypertileAction::FocusPrev)),
            KeyCode::Left | KeyCode::Char('h') => {
                Some(RuntimeAction::Core(HypertileAction::FocusDirection {
                    direction: Direction::Horizontal,
                    towards: Towards::Start,
                }))
            }
            KeyCode::Right | KeyCode::Char('l') => {
                Some(RuntimeAction::Core(HypertileAction::FocusDirection {
                    direction: Direction::Horizontal,
                    towards: Towards::End,
                }))
            }
            KeyCode::Down | KeyCode::Char('j') => {
                Some(RuntimeAction::Core(HypertileAction::FocusDirection {
                    direction: Direction::Vertical,
                    towards: Towards::End,
                }))
            }
            KeyCode::Up | KeyCode::Char('k') => {
                Some(RuntimeAction::Core(HypertileAction::FocusDirection {
                    direction: Direction::Vertical,
                    towards: Towards::Start,
                }))
            }
            KeyCode::Char('s') => Some(RuntimeAction::SplitDefault(Direction::Horizontal)),
            KeyCode::Char('v') => Some(RuntimeAction::SplitDefault(Direction::Vertical)),
            KeyCode::Char('d') => Some(RuntimeAction::Core(HypertileAction::CloseFocused)),
            KeyCode::Char('[') => Some(RuntimeAction::Core(HypertileAction::ResizeFocused {
                delta: -self.core.resize_step(),
            })),
            KeyCode::Char(']') => Some(RuntimeAction::Core(HypertileAction::ResizeFocused {
                delta: self.core.resize_step(),
            })),
            KeyCode::Char('p') => Some(RuntimeAction::OpenPalette),
            KeyCode::Char('i') => Some(RuntimeAction::EnterPluginInput),
            KeyCode::Enter => Some(RuntimeAction::InteractFocused),
            _ => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::runtime::HypertileRuntime;
    use ratatui_hypertile::MoveScope;

    #[test]
    fn default_move_bindings_include_vim_and_shift_arrows() {
        let runtime = HypertileRuntime::new();

        let shift_arrow = runtime.default_layout_action(KeyChord {
            code: KeyCode::Left,
            modifiers: Modifiers::SHIFT,
        });
        assert!(matches!(
            shift_arrow,
            Some(RuntimeAction::Core(HypertileAction::MoveFocused {
                direction: Direction::Horizontal,
                towards: Towards::Start,
                scope: MoveScope::Window,
            }))
        ));

        let vim = runtime.default_layout_action(KeyChord {
            code: KeyCode::Char('H'),
            modifiers: Modifiers::SHIFT,
        });
        assert!(matches!(
            vim,
            Some(RuntimeAction::Core(HypertileAction::MoveFocused {
                direction: Direction::Horizontal,
                towards: Towards::Start,
                scope: MoveScope::Window,
            }))
        ));
    }
}
