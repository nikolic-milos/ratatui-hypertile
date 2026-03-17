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
    HypertilePlugin, HypertileRuntime, ModeIndicator, SplitBehavior, WorkspaceRuntime,
    event_from_crossterm,
};
use std::{
    collections::VecDeque,
    io,
    time::{Duration, Instant},
};

fn build_runtime() -> HypertileRuntime {
    let mut rt = HypertileRuntime::builder()
        .with_split_behavior(SplitBehavior::Placeholder)
        .build();
    rt.register_plugin_type("monitor", || MonitorPlugin {
        cpu: [15, 42, 8, 63],
        mem: 34,
        tick: 0,
    });
    rt.register_plugin_type("logs", || LogsPlugin {
        lines: VecDeque::new(),
        tick: 0,
    });
    rt.register_plugin_type("editor", || EditorPlugin {
        text: String::new(),
    });
    rt.register_plugin_type("network", || NetworkPlugin { tick: 0 });
    rt
}

fn main() -> io::Result<()> {
    let mut terminal = ratatui::init();

    let mut workspace = WorkspaceRuntime::new(build_runtime);

    let rt = workspace.active_runtime_mut();
    let _ = rt.replace_focused_plugin("monitor");
    let _ = rt.split_focused(Direction::Vertical, "logs");
    let _ = rt.focus_pane(PaneId::ROOT);
    let _ = rt.split_focused(Direction::Horizontal, "network");

    let result = run(&mut terminal, &mut workspace);
    ratatui::restore();
    result
}

fn run(
    terminal: &mut ratatui::DefaultTerminal,
    workspace: &mut WorkspaceRuntime,
) -> io::Result<()> {
    let tick_rate = Duration::from_millis(300);
    let mut last_tick = Instant::now();

    loop {
        terminal.draw(|frame| {
            let [tabs, gap_top, body, gap_bot, footer] = Layout::vertical([
                Constraint::Length(1),
                Constraint::Length(1),
                Constraint::Min(0),
                Constraint::Length(1),
                Constraint::Length(1),
            ])
            .areas(frame.area());

            render_tabs(workspace, tabs, frame.buffer_mut());
            let _ = gap_top;
            workspace.render(body, frame.buffer_mut());
            let _ = gap_bot;

            let rt = workspace.active_runtime();
            let [mode_area, hint_area] =
                Layout::horizontal([Constraint::Length(10), Constraint::Min(0)]).areas(footer);
            ModeIndicator::new(rt.mode()).render(mode_area, frame.buffer_mut());
            Paragraph::new("  Ctrl+t/w: tab | s/v: split | d: close | p: palette | i: input")
                .style(Style::default().fg(Color::DarkGray))
                .render(hint_area, frame.buffer_mut());
        })?;

        let timeout = tick_rate.saturating_sub(last_tick.elapsed());
        if event::poll(timeout)?
            && let Event::Key(key) = event::read()?
        {
            if key.code == KeyCode::Char('c') && key.modifiers == KeyModifiers::CONTROL {
                return Ok(());
            }
            if let Some(ev) = event_from_crossterm(key) {
                workspace.handle_event(ev);
            }
        }

        if last_tick.elapsed() >= tick_rate {
            workspace.handle_event(HypertileEvent::Tick);
            last_tick = Instant::now();
        }
    }
}

fn render_tabs(workspace: &WorkspaceRuntime, area: Rect, buf: &mut Buffer) {
    let spans: Vec<Span> = workspace
        .tab_labels()
        .enumerate()
        .flat_map(|(i, (label, active))| {
            let sep = if i > 0 {
                vec![Span::raw(" ")]
            } else {
                vec![]
            };
            let tab = if active {
                Span::styled(
                    format!(" {label} "),
                    Style::default()
                        .fg(Color::Rgb(30, 30, 46))
                        .bg(Color::Rgb(137, 180, 250))
                        .bold(),
                )
            } else {
                Span::styled(
                    format!(" {label} "),
                    Style::default()
                        .fg(Color::Rgb(205, 214, 244))
                        .bg(Color::Rgb(69, 71, 90)),
                )
            };
            sep.into_iter().chain(std::iter::once(tab))
        })
        .collect();
    Line::from(spans).render(area, buf);
}

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

struct NetworkPlugin {
    tick: u64,
}

impl HypertilePlugin for NetworkPlugin {
    fn render(&self, area: Rect, buf: &mut Buffer, is_focused: bool) {
        let t = self.tick;
        let conns = 800 + (t * 17 % 120) as u32;
        let rps = 1100 + (t * 31 % 400) as u32;
        let p50 = 8 + (t * 3 % 15) as u32;
        let p99 = 60 + (t * 7 % 80) as u32;
        let errs = (t * 11 % 12) as u32;
        let up_h = t / 12;
        let up_m = (t * 5) % 60;

        let stat = |label: &str, value: String, color: Color| {
            Line::from(vec![
                Span::styled(format!("  {label:<16}"), Style::default().fg(Color::DarkGray)),
                Span::styled(value, Style::default().fg(color)),
            ])
        };

        let text = vec![
            Line::from(""),
            stat("connections", format!("{conns}"), Color::Green),
            stat("requests/s", format!("{rps}"), Color::Green),
            Line::from(""),
            stat("latency p50", format!("{p50}ms"), Color::Cyan),
            stat("latency p99", format!("{p99}ms"), if p99 > 100 { Color::Yellow } else { Color::Cyan }),
            Line::from(""),
            stat("errors/min", format!("{errs}"), if errs > 8 { Color::Red } else { Color::Green }),
            stat("uptime", format!("{up_h}h {up_m}m"), Color::DarkGray),
        ];
        Paragraph::new(text)
            .block(pane_block("Network", is_focused, Color::Blue))
            .render(area, buf);
    }

    fn on_event(&mut self, event: &HypertileEvent) -> EventOutcome {
        if matches!(event, HypertileEvent::Tick) {
            self.tick += 1;
            EventOutcome::Consumed
        } else {
            EventOutcome::Ignored
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
