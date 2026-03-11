//! Core-only tiling demo without the extras runtime.
//!
//! Keys: `s`/`v` split, `d` close, `r` reset, `hjkl`/arrows focus,
//! `HJKL` move, `[]` resize, `Tab` cycle, `q` quit.

use crossterm::event::{self, Event, KeyCode, KeyModifiers};
use ratatui::{
    buffer::Buffer,
    layout::Direction,
    style::{Color, Style},
    symbols::border,
    widgets::{Block, Borders, Widget},
};
use ratatui_hypertile::{
    Hypertile, HypertileAction, HypertileWidget, MoveScope, PaneId, PaneSnapshot, Towards,
};
use std::{collections::BTreeMap, io, time::Duration};

const COLORS: [Color; 6] = [
    Color::Cyan,
    Color::Green,
    Color::Yellow,
    Color::Blue,
    Color::Magenta,
    Color::Red,
];

fn main() -> io::Result<()> {
    let mut terminal = ratatui::init();
    let mut layout = Hypertile::new();
    let mut labels: BTreeMap<PaneId, (String, Color)> = BTreeMap::new();
    labels.insert(PaneId::ROOT, ("Pane 1".into(), COLORS[0]));
    let mut count = 1usize;

    loop {
        terminal.draw(|frame| {
            frame.render_stateful_widget(
                HypertileWidget::new(|pane, buf| render_pane(pane, buf, &labels)),
                frame.area(),
                &mut layout,
            );
        })?;

        if !event::poll(Duration::from_millis(100))? {
            continue;
        }
        let Event::Key(key) = event::read()? else {
            continue;
        };

        let none = key.modifiers == KeyModifiers::NONE;
        let shift = key.modifiers == KeyModifiers::SHIFT;

        match key.code {
            KeyCode::Char('q') if none => break,
            KeyCode::Char('c') if key.modifiers.contains(KeyModifiers::CONTROL) => break,

            KeyCode::Char('s') if none => {
                split(&mut layout, &mut labels, &mut count, Direction::Horizontal)
            }
            KeyCode::Char('v') if none => {
                split(&mut layout, &mut labels, &mut count, Direction::Vertical)
            }
            KeyCode::Char('d') if none => {
                if let Ok(id) = layout.close_focused() {
                    labels.remove(&id);
                }
            }
            KeyCode::Char('r') if none => {
                layout.reset();
                labels.clear();
                labels.insert(PaneId::ROOT, ("Pane 1".into(), COLORS[0]));
                count = 1;
            }

            KeyCode::Tab => {
                layout.apply_action(HypertileAction::FocusNext);
            }
            KeyCode::BackTab => {
                layout.apply_action(HypertileAction::FocusPrev);
            }

            KeyCode::Left | KeyCode::Char('h') if none => {
                focus(&mut layout, Direction::Horizontal, Towards::Start)
            }
            KeyCode::Right | KeyCode::Char('l') if none => {
                focus(&mut layout, Direction::Horizontal, Towards::End)
            }
            KeyCode::Up | KeyCode::Char('k') if none => {
                focus(&mut layout, Direction::Vertical, Towards::Start)
            }
            KeyCode::Down | KeyCode::Char('j') if none => {
                focus(&mut layout, Direction::Vertical, Towards::End)
            }

            KeyCode::Char('H') if shift => {
                move_pane(&mut layout, Direction::Horizontal, Towards::Start)
            }
            KeyCode::Char('L') if shift => {
                move_pane(&mut layout, Direction::Horizontal, Towards::End)
            }
            KeyCode::Char('K') if shift => {
                move_pane(&mut layout, Direction::Vertical, Towards::Start)
            }
            KeyCode::Char('J') if shift => {
                move_pane(&mut layout, Direction::Vertical, Towards::End)
            }

            KeyCode::Char('[') if none => {
                layout.apply_action(HypertileAction::ResizeFocused { delta: -0.05 });
            }
            KeyCode::Char(']') if none => {
                layout.apply_action(HypertileAction::ResizeFocused { delta: 0.05 });
            }

            _ => {}
        }
    }

    ratatui::restore();
    Ok(())
}

fn split(
    layout: &mut Hypertile,
    labels: &mut BTreeMap<PaneId, (String, Color)>,
    count: &mut usize,
    dir: Direction,
) {
    if let Ok(id) = layout.split_focused(dir) {
        labels.insert(
            id,
            (
                format!("Pane {}", *count + 1),
                COLORS[*count % COLORS.len()],
            ),
        );
        *count += 1;
    }
}

fn focus(layout: &mut Hypertile, direction: Direction, towards: Towards) {
    layout.apply_action(HypertileAction::FocusDirection { direction, towards });
}

fn move_pane(layout: &mut Hypertile, direction: Direction, towards: Towards) {
    layout.apply_action(HypertileAction::MoveFocused {
        direction,
        towards,
        scope: MoveScope::Window,
    });
}

fn render_pane(pane: PaneSnapshot, buf: &mut Buffer, labels: &BTreeMap<PaneId, (String, Color)>) {
    let (title, color) = labels
        .get(&pane.id)
        .map(|(t, c)| (t.as_str(), *c))
        .unwrap_or(("Pane", Color::White));

    let mut block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(color))
        .title(title);
    if pane.is_focused {
        block = block
            .border_set(border::THICK)
            .border_style(Style::default().fg(color).bold());
    }
    block.render(pane.rect, buf);
}
