use crossterm::{
    event::{self, Event, KeyCode, KeyEventKind},
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
    ExecutableCommand,
};
use ratatui::{prelude::*, widgets::*};
use std::io::stdout;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Duration;

pub async fn select_menu(options: Vec<&str>) -> usize {
    enable_raw_mode().unwrap();
    stdout().execute(EnterAlternateScreen).unwrap();
    let mut terminal = Terminal::new(CrosstermBackend::new(stdout())).unwrap();
    let mut default_index = 0;
    loop {
        terminal.draw(|f| {
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
            let list = List::new(items).block(Block::default().title(title).borders(Borders::ALL));
            f.render_widget(list, area);
        }).unwrap();

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

pub async fn dashboard(stt_enabled: Arc<AtomicBool>, mut rx_out: tokio::sync::mpsc::UnboundedReceiver<String>) {
    enable_raw_mode().unwrap();
    stdout().execute(EnterAlternateScreen).unwrap();
    let mut terminal = Terminal::new(CrosstermBackend::new(stdout())).unwrap();
    let mut chat_history: Vec<String> = Vec::new();
    loop {
        terminal.draw(|f| {
            let stt_status_text = if stt_enabled.load(Ordering::Relaxed) {
                "STT Status: Muted (Press 'm' to unmute)"
            } else {
                "STT Status: Unmuted (Press 'm' to mute)"
            };
            let chunks = Layout::default()
                .direction(Direction::Vertical)
                .constraints([Constraint::Length(3), Constraint::Min(3)])
                .split(f.area());
            let stt_show_text = Paragraph::new(stt_status_text)
                .style(if stt_enabled.load(Ordering::Relaxed) {
                    Style::default().fg(Color::Red)
                } else {
                    Style::default().fg(Color::Green)
                })
                .block(Block::default().borders(Borders::ALL).title("Talos Dashboard"));
            f.render_widget(stt_show_text, chunks[0]);
            
            let items: Vec<ListItem> = chat_history.iter().map(|msg| ListItem::new(msg.as_str())).collect();
            let list = List::new(items).block(Block::default().borders(Borders::ALL).title("Chat"));
            let mut list_state = ListState::default();
            if !chat_history.is_empty() {
                list_state.select(Some(chat_history.len() - 1));
            }
            f.render_stateful_widget(list, chunks[1], &mut list_state);
        }).unwrap();
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
            chat_history.push(message);
        }
    }
    disable_raw_mode().unwrap();
    stdout().execute(LeaveAlternateScreen).unwrap();
}