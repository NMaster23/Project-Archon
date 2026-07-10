use crossterm::{
    ExecutableCommand,
    event::{self, Event, KeyCode, KeyEventKind},
    terminal::{EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode},
};
use ratatui::{prelude::*, widgets::*};
use std::io::stdout;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::thread;
use std::time::{Duration, Instant};

const DEFAULT_FRAMES: &[&str] = &["⠋", "⠙", "⠹", "⠸", "⠼", "⠴", "⠦", "⠧", "⠇", "⠏"];

pub struct Spinner {
    frames: &'static [&'static str],
    index: usize,
    last_tick: Instant,
    tick_rate: Duration,
}

impl Spinner {
    pub fn new(tick_rate_ms: u64) -> Self {
        Self {
            frames: DEFAULT_FRAMES,
            index: 0,
            last_tick: Instant::now(),
            tick_rate: Duration::from_millis(tick_rate_ms),
        }
    }
    pub fn with_custom_frames(tick_rate_ms: u64, frames: &'static [&'static str]) -> Self {
        Self {
            frames,
            index: 0,
            last_tick: Instant::now(),
            tick_rate: Duration::from_millis(tick_rate_ms),
        }
    }
    pub fn update(&mut self) {
        if self.last_tick.elapsed() >= self.tick_rate {
            self.index = (self.index + 1) % self.frames.len();
            self.last_tick = Instant::now();
        }
    }
    pub fn frame(&self) -> &str {
        self.frames[self.index]
    }
    pub fn reset(&mut self) {
        self.index = 0;
        self.last_tick = Instant::now();
    }
}

pub async fn select_menu(options: Vec<&str>) -> usize {
    enable_raw_mode().unwrap();
    stdout().execute(EnterAlternateScreen).unwrap();
    let mut terminal = Terminal::new(CrosstermBackend::new(stdout())).unwrap();
    let mut default_index = 0;
    loop {
        terminal
            .draw(|f| {
                let term_area = f.area();
                let max_width = options.iter().map(|s| s.len()).max().unwrap_or(0);
                let title = "Select an option";
                let width = (max_width.max(title.len()) + 4) as u16;
                let height = (options.len() + 2) as u16;
                let area = Rect::new(
                    term_area.x,
                    term_area.y,
                    width.min(term_area.width),
                    height.min(term_area.height),
                );

                let items: Vec<ListItem> = options
                    .iter()
                    .enumerate()
                    .map(|(i, option)| {
                        let style = if i == default_index {
                            Style::default().fg(Color::Yellow)
                        } else {
                            Style::default()
                        };
                        ListItem::new(*option).style(style)
                    })
                    .collect();
                let list =
                    List::new(items).block(Block::default().title(title).borders(Borders::ALL));
                f.render_widget(list, area);
            })
            .unwrap();

        if let Event::Key(key) = event::read().unwrap() {
            if key.kind == KeyEventKind::Press {
                match key.code {
                    KeyCode::Up => {
                        if default_index > 0 {
                            default_index -= 1;
                        }
                    }
                    KeyCode::Down => {
                        if default_index < options.len() - 1 {
                            default_index += 1;
                        }
                    }
                    KeyCode::Enter => {
                        break;
                    }
                    _ => {}
                }
            }
        }
    }

    disable_raw_mode().unwrap();
    stdout().execute(LeaveAlternateScreen).unwrap();
    default_index
}

pub async fn dashboard(
    stt_enabled: Arc<AtomicBool>,
    mut rx_out: tokio::sync::mpsc::UnboundedReceiver<String>,
) {
    enable_raw_mode().unwrap();
    stdout().execute(EnterAlternateScreen).unwrap();
    let mut terminal = Terminal::new(CrosstermBackend::new(stdout())).unwrap();
    let mut chat_history: Vec<String> = Vec::new();
    let mut spinner = Spinner::new(100);
    let mut processing = false;
    loop { spinner.update();
        terminal
            .draw(|f| {
                let stt_status_text = if stt_enabled.load(Ordering::Relaxed) {
                    "STT Status: Muted (Press 'm' to unmute)"
                } else {
                    "STT Status: Unmuted (Press 'm' to mute)"
                };
                let chunks = Layout::default()
                    .direction(Direction::Vertical)
                    .constraints([Constraint::Length(3), Constraint::Min(3)])
                    .split(f.area());
                let display_status_text = if processing { format!("{} {}", spinner.frame(), stt_status_text) } else { stt_status_text.to_string() };
                let stt_show_text = Paragraph::new(display_status_text)
                    .style(if stt_enabled.load(Ordering::Relaxed) {
                        Style::default().fg(Color::Red)
                    } else {
                        Style::default().fg(Color::Green)
                    })
                    .block(
                        Block::default()
                            .borders(Borders::ALL)
                            .title("Talos Dashboard"),
                    );
                f.render_widget(stt_show_text, chunks[0]);

                let chat_text = chat_history.join("\n");
                let paragraph = Paragraph::new(chat_text)
                    .block(Block::default().borders(Borders::ALL).title("Chat"))
                    .wrap(Wrap { trim: true });
                f.render_widget(paragraph, chunks[1]);
            })
            .unwrap();
        if event::poll(Duration::from_millis(16)).unwrap() {
            if let Event::Key(key) = event::read().unwrap() {
                if key.kind == KeyEventKind::Press {
                    match key.code {
                        KeyCode::Char('m') => {
                            let current_state = stt_enabled.load(Ordering::Relaxed);
                            stt_enabled.store(!current_state, Ordering::Relaxed);
                        }
                        KeyCode::Char('q') => break,
                        _ => {}
                    }
                }
            }
        }
        while let Ok(message) = rx_out.try_recv() {
            match message.as_str() {
                "__PROCESSING_START__" => {
                    processing = true;
                    spinner.reset();
                    break;
                }
                "__PROCESSING_END__" => {
                    processing = false;
                    break;
                }
                other => chat_history.push(other.to_string()),
            }
        }
        thread::sleep(Duration::from_millis(16));
    }
    disable_raw_mode().unwrap();
    stdout().execute(LeaveAlternateScreen).unwrap();
}
