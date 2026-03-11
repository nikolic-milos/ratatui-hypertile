//! Extras runtime demo with four plugin types: monitor, logs, editor, help.
//!
//! Uses [`HypertileRuntime`] for modal input, plugin palette, and tick-driven
//! updates.

use crossterm::event::{self, Event, KeyCode, KeyModifiers};
use ratatui::{
    buffer::Buffer,
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Style},
    symbols::border,
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph, Widget, Wrap},
};
use ratatui_hypertile::{EventOutcome, HypertileEvent, KeyCode as HtKeyCode, PaneId};
use ratatui_hypertile_extras::{
    HypertilePlugin, HypertileRuntime, InputMode, SplitBehavior, event_from_crossterm,
};
use std::{
    collections::VecDeque,
    io,
    time::{Duration, Instant},
};

struct MonitorPlugin {
    cpu: [u8; 4],
    mem: u8,
    tick: u64,
}

impl HypertilePlugin for MonitorPlugin {
    fn render(&self, area: Rect, buf: &mut Buffer, is_focused: bool) {
        let mut lines = vec![Line::from("")];
        for (i, &usage) in self.cpu.iter().enumerate() {
            let filled = usage as usize * 20 / 100;
            let bar = "\u{2588}".repeat(filled) + &"\u{2591}".repeat(20 - filled);
            let color = match usage {
                0..50 => Color::Green,
                50..80 => Color::Yellow,
                _ => Color::Red,
            };
            lines.push(Line::from(vec![
                Span::raw(format!("  cpu{i} ")),
                Span::styled(bar, Style::default().fg(color)),
                Span::raw(format!(" {:>3}%", usage)),
            ]));
        }
        lines.push(Line::from(""));
        let filled = self.mem as usize * 20 / 100;
        let bar = "\u{2588}".repeat(filled) + &"\u{2591}".repeat(20 - filled);
        lines.push(Line::from(vec![
            Span::raw("  mem  "),
            Span::styled(bar, Style::default().fg(Color::Cyan)),
            Span::raw(format!(" {:.1}G/16G", self.mem as f64 * 16.0 / 100.0)),
        ]));

        Paragraph::new(lines)
            .block(pane_block("Monitor", is_focused, Color::Green))
            .render(area, buf);
    }

    fn on_event(&mut self, event: &HypertileEvent) -> EventOutcome {
        if !matches!(event, HypertileEvent::Tick) {
            return EventOutcome::Ignored;
        }
        self.tick += 1;
        let t = self.tick;
        self.cpu[0] = ((t * 7 + 15) % 85 + 10) as u8;
        self.cpu[1] = ((t * 13 + 42) % 75 + 5) as u8;
        self.cpu[2] = ((t * 3 + 28) % 90 + 8) as u8;
        self.cpu[3] = ((t * 11 + 55) % 70 + 15) as u8;
        self.mem = ((t * 2 + 34) % 30 + 40) as u8;
        EventOutcome::Consumed
    }
}

const LOG_ENTRIES: &[(&str, Color)] = &[
    ("GET /api/users 200 OK", Color::Green),
    ("POST /api/auth 201 Created", Color::Green),
    ("WARN connection pool near capacity", Color::Yellow),
    ("GET /api/health 200 OK", Color::DarkGray),
    ("ERROR failed to reach upstream", Color::Red),
    ("INFO worker 3 started", Color::Cyan),
    ("DEBUG cache hit ratio: 0.94", Color::DarkGray),
    ("GET /api/items?page=2 200 OK", Color::Green),
    ("WARN slow query: 342ms", Color::Yellow),
    ("INFO deployment v1.4.2 rolling out", Color::Cyan),
];

struct LogsPlugin {
    lines: VecDeque<(String, Color)>,
    tick: u64,
}

impl HypertilePlugin for LogsPlugin {
    fn render(&self, area: Rect, buf: &mut Buffer, is_focused: bool) {
        let text: Vec<Line> = self
            .lines
            .iter()
            .map(|(msg, color)| Line::styled(msg.as_str(), Style::default().fg(*color)))
            .collect();
        Paragraph::new(text)
            .block(pane_block("Logs", is_focused, Color::Yellow))
            .wrap(Wrap { trim: false })
            .render(area, buf);
    }

    fn on_event(&mut self, event: &HypertileEvent) -> EventOutcome {
        if !matches!(event, HypertileEvent::Tick) {
            return EventOutcome::Ignored;
        }
        self.tick += 1;
        let (msg, color) = LOG_ENTRIES[self.tick as usize % LOG_ENTRIES.len()];
        let h = (self.tick / 3600) % 24;
        let m = (self.tick / 60) % 60;
        let s = self.tick % 60;
        if self.lines.len() >= 100 {
            self.lines.pop_front();
        }
        self.lines
            .push_back((format!("{h:02}:{m:02}:{s:02} {msg}"), color));
        EventOutcome::Consumed
    }
}

struct EditorPlugin {
    text: String,
}

impl HypertilePlugin for EditorPlugin {
    fn render(&self, area: Rect, buf: &mut Buffer, is_focused: bool) {
        Paragraph::new(format!("{}\u{2588}", self.text))
            .block(pane_block("Editor", is_focused, Color::Magenta))
            .wrap(Wrap { trim: false })
            .render(area, buf);
    }

    fn on_event(&mut self, event: &HypertileEvent) -> EventOutcome {
        let HypertileEvent::Key(key) = event else {
            return EventOutcome::Ignored;
        };
        match key.code {
            HtKeyCode::Char(ch) => {
                self.text.push(ch);
                EventOutcome::Consumed
            }
            HtKeyCode::Enter => {
                self.text.push('\n');
                EventOutcome::Consumed
            }
            HtKeyCode::Backspace => {
                self.text.pop();
                EventOutcome::Consumed
            }
            _ => EventOutcome::Ignored,
        }
    }
}

struct HelpPlugin;

impl HypertilePlugin for HelpPlugin {
    fn render(&self, area: Rect, buf: &mut Buffer, is_focused: bool) {
        let text = vec![
            Line::from(""),
            Line::from("  s/v     split"),
            Line::from("  d       close pane"),
            Line::from("  p       open palette"),
            Line::from("  Enter   interact with pane"),
            Line::from("  Esc     toggle layout/input mode"),
            Line::from("  hjkl    focus direction"),
            Line::from("  HJKL    move pane"),
            Line::from("  [ ]     resize"),
            Line::from("  Tab     cycle focus"),
            Line::from("  q       quit"),
        ];
        Paragraph::new(text)
            .block(pane_block("Help", is_focused, Color::Cyan))
            .render(area, buf);
    }
}

fn main() -> io::Result<()> {
    let mut terminal = ratatui::init();

    let mut runtime = HypertileRuntime::builder()
        .with_split_behavior(SplitBehavior::Placeholder)
        .build();
    runtime.register_plugin_type("monitor", || MonitorPlugin {
        cpu: [15, 42, 8, 63],
        mem: 34,
        tick: 0,
    });
    runtime.register_plugin_type("logs", || LogsPlugin {
        lines: VecDeque::new(),
        tick: 0,
    });
    runtime.register_plugin_type("editor", || EditorPlugin {
        text: String::new(),
    });
    runtime.register_plugin_type("help", || HelpPlugin);

    let _ = runtime.replace_focused_plugin("monitor");
    let _ = runtime.split_focused(Direction::Vertical, "logs");
    let _ = runtime.focus_pane(PaneId::ROOT);
    let _ = runtime.split_focused(Direction::Horizontal, "editor");

    let result = run(&mut terminal, &mut runtime);
    ratatui::restore();
    result
}

fn run(terminal: &mut ratatui::DefaultTerminal, runtime: &mut HypertileRuntime) -> io::Result<()> {
    let tick_rate = Duration::from_millis(300);
    let mut last_tick = Instant::now();

    loop {
        terminal.draw(|frame| {
            let [header, body] =
                Layout::vertical([Constraint::Length(3), Constraint::Min(0)]).areas(frame.area());

            let mode = match runtime.mode() {
                InputMode::Layout => "layout",
                InputMode::PluginInput => "input",
            };
            Paragraph::new(format!(
                "mode: {mode} | s/v: split | d: close | p: palette | Esc: toggle mode | q: quit"
            ))
            .block(Block::default().borders(Borders::ALL).title("basic"))
            .render(header, frame.buffer_mut());

            runtime.render(body, frame.buffer_mut());
        })?;

        let timeout = tick_rate.saturating_sub(last_tick.elapsed());
        if event::poll(timeout)?
            && let Event::Key(key) = event::read()?
        {
            match (key.code, key.modifiers) {
                (KeyCode::Char('q'), KeyModifiers::NONE)
                | (KeyCode::Char('c'), KeyModifiers::CONTROL) => return Ok(()),
                _ => {
                    if let Some(ev) = event_from_crossterm(key) {
                        runtime.handle_event(ev);
                    }
                }
            }
        }

        if last_tick.elapsed() >= tick_rate {
            runtime.handle_event(HypertileEvent::Tick);
            last_tick = Instant::now();
        }
    }
}

fn pane_block<'a>(title: &'a str, is_focused: bool, color: Color) -> Block<'a> {
    if is_focused {
        Block::default()
            .borders(Borders::ALL)
            .border_set(border::THICK)
            .border_style(Style::default().fg(color).bold())
            .title(title)
    } else {
        Block::default().borders(Borders::ALL).title(title)
    }
}
