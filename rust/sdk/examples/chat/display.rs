use crate::{cmdline::Identity, messages::Message};
use acto::{
    variable::{Reader, Writer},
    ActoCell, ActoInput, ActoRuntime,
};
use chrono::{DateTime, Utc};
use ratatui::{
    prelude::{Alignment, Constraint, CrosstermBackend, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, Paragraph, Wrap},
    Frame, Terminal,
};
use std::io;
use tui_textarea::TextArea;

mod guard {
    use crossterm::{
        event::{DisableMouseCapture, EnableMouseCapture},
        execute,
        terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
    };
    use std::{io, marker::PhantomData};

    pub struct TermGuard(PhantomData<u8>);

    impl TermGuard {
        pub fn new() -> TermGuard {
            tracing::info!("Initializing TermGuard");
            enable_raw_mode().expect("failed to enable raw mode");
            execute!(io::stdout(), EnterAlternateScreen, EnableMouseCapture).expect("failed to enter alternate screen");
            TermGuard(PhantomData)
        }
    }

    impl Drop for TermGuard {
        fn drop(&mut self) {
            tracing::info!("Dropping TermGuard");
            reset_terminal();
        }
    }

    pub fn reset_terminal() {
        disable_raw_mode().expect("failed to disable raw mode");
        execute!(io::stdout(), LeaveAlternateScreen, DisableMouseCapture).expect("failed to leave alternate screen");
    }
}

pub use guard::reset_terminal;

pub enum Display {
    Cmdline(Reader<TextArea<'static>>),
    Messages(Reader<Vec<Message>>),
    NotConnected(String),
    UpdateIdentity(Reader<Identity>),
    Connected,
}

pub async fn display(mut cell: ActoCell<Display, impl ActoRuntime>) {
    let _guard = guard::TermGuard::new();
    let mut terminal = Terminal::new(CrosstermBackend::new(io::stdout())).expect("failed to create terminal");

    let mut identity = Writer::new(Identity::default()).reader();
    let mut text_area = Writer::new(TextArea::default()).reader();
    let mut messages = Writer::new(Vec::new()).reader();
    let mut not_connected = None;

    while let ActoInput::Message(msg) = cell.recv().await {
        match msg {
            Display::Cmdline(t) => text_area = t,
            Display::Messages(m) => messages = m,
            Display::NotConnected(e) => not_connected = Some(e),
            Display::Connected => not_connected = None,
            Display::UpdateIdentity(i) => identity = i,
        }
        let res = terminal.draw(|f| {
            render(f, &identity, &text_area, &messages, &not_connected);
        });
        if let Err(e) = res {
            tracing::error!("failed to draw terminal: {}", e);
            return;
        }
    }
}

fn render<W: io::Write>(
    f: &mut Frame<CrosstermBackend<W>>,
    identity: &Reader<Identity>,
    text_area: &Reader<TextArea>,
    messages: &Reader<Vec<Message>>,
    not_connected: &Option<String>,
) {
    identity.project(|identity| match &identity.edit {
        Some(edit) => render_editing_identity(f, edit),
        None => render_chat(f, text_area, messages, not_connected),
    });
}

fn render_editing_identity<W: io::Write>(f: &mut Frame<CrosstermBackend<W>>, edit: &TextArea) {
    let size = f.size();
    let layout = Layout::default()
        .direction(ratatui::prelude::Direction::Vertical)
        .constraints([ratatui::prelude::Constraint::Length(1)])
        .split(size);

    f.render_widget(edit.widget(), layout[0]);
}

fn render_chat<W: io::Write>(
    f: &mut Frame<CrosstermBackend<W>>,
    text_area: &Reader<TextArea>,
    messages: &Reader<Vec<Message>>,
    not_connected: &Option<String>,
) {
    let size = f.size();
    let cmdheight = text_area.project(|t| t.lines().len() as u16);
    let layout = Layout::default()
        .direction(ratatui::prelude::Direction::Vertical)
        .constraints([
            ratatui::prelude::Constraint::Min(0),
            ratatui::prelude::Constraint::Length(cmdheight + 2),
        ])
        .split(size);

    messages.project(|msgs| {
        let lines = msgs.into_iter().map(message_to_line).collect::<Vec<_>>();
        f.render_widget(Paragraph::new(lines), layout[0]);
    });

    text_area.project(|t| f.render_widget(t.widget(), layout[1]));

    if let Some(ref e) = not_connected {
        let rect = centered_rect(60, 20, size);
        let text = Paragraph::new(vec![
            Line::from(Span::styled(
                "Not connected to Actyx",
                Style::new().add_modifier(Modifier::BOLD),
            )),
            Line::from(""),
            Line::from(e.as_str()),
        ])
        .alignment(Alignment::Center)
        .wrap(Wrap { trim: true })
        .block(
            Block::default()
                .title("Error")
                .borders(Borders::ALL)
                .style(Style::new().bg(Color::Red)),
        );
        f.render_widget(Clear, rect);
        f.render_widget(text, rect);
    }
}

fn message_to_line(msg: &Message) -> Line {
    let time = DateTime::<Utc>::from(msg.time).to_rfc3339();
    Line::from(vec![
        Span::styled(time, Style::new().add_modifier(Modifier::ITALIC)),
        Span::raw(" "),
        Span::styled(&msg.from, Style::new().add_modifier(Modifier::BOLD)),
        Span::raw(": "),
        Span::raw(&msg.text),
    ])
}

/// helper function to create a centered rect using up certain percentage of the available rect `r`
fn centered_rect(percent_x: u16, percent_y: u16, r: Rect) -> Rect {
    let popup_layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Percentage((100 - percent_y) / 2),
            Constraint::Percentage(percent_y),
            Constraint::Percentage((100 - percent_y) / 2),
        ])
        .split(r);

    Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage((100 - percent_x) / 2),
            Constraint::Percentage(percent_x),
            Constraint::Percentage((100 - percent_x) / 2),
        ])
        .split(popup_layout[1])[1]
}
