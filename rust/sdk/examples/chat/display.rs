use crate::messages::Message;
use acto::{
    variable::{Reader, Writer},
    ActoCell, ActoInput, ActoRuntime,
};
use chrono::{DateTime, Utc};
use ratatui::{
    prelude::{CrosstermBackend, Layout},
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::Paragraph,
    Terminal,
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
}

pub async fn display(mut cell: ActoCell<Display, impl ActoRuntime>) {
    let _guard = guard::TermGuard::new();
    let mut terminal = Terminal::new(CrosstermBackend::new(io::stdout())).expect("failed to create terminal");

    let mut text_area = Writer::new(TextArea::default()).reader();
    let mut messages = Writer::new(Vec::new()).reader();

    while let ActoInput::Message(msg) = cell.recv().await {
        match msg {
            Display::Cmdline(t) => text_area = t,
            Display::Messages(m) => messages = m,
        }
        let res = terminal.draw(|f| {
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
                let mut lines = vec![];
                for msg in msgs {
                    let time = DateTime::<Utc>::from(msg.time).to_rfc3339();
                    lines.push(Line::from(vec![
                        Span::styled(time, Style::new().add_modifier(Modifier::ITALIC)),
                        Span::raw(" "),
                        Span::styled(&msg.from, Style::new().add_modifier(Modifier::BOLD)),
                        Span::raw(": "),
                        Span::raw(&msg.text),
                    ]));
                }
                f.render_widget(Paragraph::new(lines), layout[0]);
            });

            text_area.project(|t| f.render_widget(t.widget(), layout[1]));
        });
        if let Err(e) = res {
            tracing::error!("failed to draw terminal: {}", e);
            return;
        }
    }
}
