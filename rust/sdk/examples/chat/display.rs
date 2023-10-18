use crate::{cmdline::Identity, messages::Message};
use acto::{
    variable::{Reader, Writer},
    ActoCell, ActoInput, ActoRuntime,
};
use chrono::{DateTime, Utc};
use ratatui::{
    prelude::{Alignment, Constraint, CrosstermBackend, Direction, Layout, Rect},
    style::{Color, Modifier, Style, Stylize},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, Paragraph, Wrap},
    Frame, Terminal,
};
use std::{collections::VecDeque, convert::TryInto, io};
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
    Connected,
    UpdateIdentity(Reader<Identity>),
    Scroll(i8),
}

impl Display {
    pub fn scroll_down(multiplier: i8) -> Self {
        Display::Scroll(-1 * multiplier)
    }
    pub fn scroll_up(multiplier: i8) -> Self {
        Display::Scroll(1 * multiplier)
    }
}

#[derive(Default)]
struct MessageScrollState {
    /// vertical_scroll_from_the_bottom
    pub last_text_height: Option<u16>,
    pub vscroll_from_bottom: u16,
}

impl MessageScrollState {
    pub fn update_text_height_and_clamp_vscroll(&mut self, text_height: u16, clamp_max_vscroll: u16) {
        // if scroll_state.vscroll_from_bottom is not 0, it must shift up, because UX
        if self.vscroll_from_bottom > 0 {
            // Is there a height shift from previous render?
            let height_shift_from_previous_render = self
                .last_text_height
                .map_or(0_u16, |prev_height| text_height.saturating_sub(prev_height));

            self.vscroll_from_bottom = self
                .vscroll_from_bottom
                .saturating_add(height_shift_from_previous_render)
                .clamp(0, clamp_max_vscroll);
        }

        self.last_text_height = Some(text_height);
    }
}

pub async fn display(mut cell: ActoCell<Display, impl ActoRuntime>) {
    let _guard = guard::TermGuard::new();
    let mut terminal = Terminal::new(CrosstermBackend::new(io::stdout())).expect("failed to create terminal");

    let mut identity = Writer::new(Identity::default()).reader();
    let mut text_area = Writer::new(TextArea::default()).reader();
    let mut messages = Writer::new(Vec::new()).reader();
    let mut scroll_state = MessageScrollState::default();
    let mut not_connected = None;

    while let ActoInput::Message(msg) = cell.recv().await {
        match msg {
            Display::Cmdline(t) => text_area = t,
            Display::Messages(m) => messages = m,
            Display::NotConnected(e) => not_connected = Some(e),
            Display::Connected => not_connected = None,
            Display::UpdateIdentity(i) => identity = i,
            Display::Scroll(i) => {
                let scroll_val = if i >= 0 {
                    scroll_state.vscroll_from_bottom.saturating_add(i as u16)
                } else {
                    scroll_state.vscroll_from_bottom.saturating_sub((-i) as u16)
                };

                scroll_state.vscroll_from_bottom = scroll_val;
            }
        }
        let res = terminal.draw(|f| {
            render(f, &identity, &text_area, &messages, &mut scroll_state, &not_connected);
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
    scroll_state: &mut MessageScrollState,
    not_connected: &Option<String>,
) {
    let is_editing = identity.project(|identity| identity.edit.is_some());
    if is_editing {
        render_editing_identity(f, identity);
    } else {
        render_chat(f, text_area, messages, scroll_state, not_connected);
    }
}

fn render_editing_identity<W: io::Write>(f: &mut Frame<CrosstermBackend<W>>, identity: &Reader<Identity>) {
    let size = f.size();
    let pad = (size.height / 2).saturating_sub(2);
    let layout = Layout::default()
        .direction(ratatui::prelude::Direction::Vertical)
        .constraints([
            ratatui::prelude::Constraint::Length(pad),
            ratatui::prelude::Constraint::Length(3),
            ratatui::prelude::Constraint::Min(0),
        ])
        .split(size);

    identity.project(|identity| {
        if let Some(edit) = &identity.edit {
            f.render_widget(edit.widget(), layout[1]);
        }
    });
}

fn render_chat<W: io::Write>(
    f: &mut Frame<CrosstermBackend<W>>,
    text_area: &Reader<TextArea>,
    messages: &Reader<Vec<Message>>,
    scroll_state: &mut MessageScrollState,
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
        let message_text_rect = layout[0];
        let message_text_rect_width = message_text_rect.width;
        let message_text_rect_height = message_text_rect.height;
        tracing::info!("paragraph_available_width: {}", message_text_rect_width);
        let mut lines = msgs
            .into_iter()
            .flat_map(|x| message_to_lines(x, message_text_rect_width))
            .collect::<Vec<_>>();

        // FIXME Maybe there is a better way to calculate height using Widget::render, Buffer, and binary search?
        // That way using Paragraph::wrap can be possible
        let text_height: u16 = lines.len().try_into().unwrap_or(0);
        let max_scroll = text_height.saturating_sub(message_text_rect_height);

        scroll_state.update_text_height_and_clamp_vscroll(text_height, max_scroll);

        // calculate scroll_y from top
        let scroll_y = max_scroll.saturating_sub(scroll_state.vscroll_from_bottom);

        // "render" scroll control key by mutating the top-most-rendered line and bottom-most-rendered-line
        if text_height > message_text_rect_height {
            // when not at the bottom
            if scroll_state.vscroll_from_bottom != 0 {
                let visible_bottom = scroll_y + message_text_rect_height - 1;
                if let Some(last_line) = lines.get_mut(visible_bottom as usize) {
                    *last_line = Line::from(vec![Span::from("Press PgDn to scroll down (Hold Ctrl to Boost)")])
                        .alignment(Alignment::Center);
                }
            }

            // when not at the top
            if scroll_y != 0 {
                let visible_top = scroll_y;
                if let Some(first_line) = lines.get_mut(visible_top as usize) {
                    *first_line = Line::from(vec![Span::from("Press PgUp to scroll up (Hold Ctrl to Boost)")])
                        .alignment(Alignment::Center);
                }
            }
        }

        let paragraph = Paragraph::new(lines).dark_gray().scroll((scroll_y, 0));

        f.render_widget(paragraph, message_text_rect);
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

// FIXME sometimes, Scrollbar from ratatui doesn't work for bottom-to-top scrolling
//  because there is no way to calculate the height of a text
//  meanwhile Paragraph::wrap has an opaque wrapping mechanism
fn split_into_lines(spans: VecDeque<Span>, max_width: u16) -> Vec<Line> {
    let max_width_as_usize: usize = max_width.into();
    let mut lines = vec![];
    let mut current_line = Line::default();
    let mut current_line_width: usize = 0;

    for span in spans {
        let Span { content, style } = span;
        let mut content: String = content.into();
        while content.len() > 0 {
            let remaining_line_width = max_width_as_usize.saturating_sub(current_line_width);

            if remaining_line_width == 0 {
                let line = std::mem::replace(&mut current_line, Line::default());
                lines.push(line);
                current_line_width = 0;
            } else {
                let split_point = usize::min(remaining_line_width, content.len());
                let (pushable_content, remaining_content) = {
                    let (a, b) = content.split_at(split_point);
                    (String::from(a), String::from(b))
                };

                let pushable_content_length = pushable_content.len();
                let mut new_span = Span::from(pushable_content);
                new_span.patch_style(style.clone());

                current_line.spans.push(new_span);
                current_line_width += pushable_content_length;

                content = remaining_content
            }
        }
    }

    lines.push(current_line);

    lines
}

fn message_to_lines(msg: &Message, max_width: u16) -> Vec<Line> {
    let time = DateTime::<Utc>::from(msg.time).to_rfc3339();
    split_into_lines(
        VecDeque::from([
            Span::styled(time, Style::new().add_modifier(Modifier::ITALIC)),
            Span::raw(" "),
            Span::styled(&msg.from, Style::new().add_modifier(Modifier::BOLD)),
            Span::raw(": "),
            Span::raw(&msg.text),
        ]),
        max_width,
    )
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
