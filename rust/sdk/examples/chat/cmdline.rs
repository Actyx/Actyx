use crate::{
    display::Display,
    messages::{self, Messages},
};
use acto::{variable::Writer, ActoCell, ActoInput, ActoRef, ActoRuntime};
use crossterm::event::{Event, KeyCode, KeyEvent, KeyEventKind, KeyModifiers};
use ratatui::widgets::{Block, Borders};
use tui_textarea::TextArea;

pub enum Cmdline {
    Event(Event),
    Reconnect(ActoRef<Messages>),
}

fn mk_message_text_area() -> TextArea<'static> {
    let mut text_area = TextArea::default();
    text_area.set_block(
        Block::default()
            .title("your message (Enter to send, Ctrl-I to Edit Identity, Ctrl-C to quit)")
            .borders(Borders::ALL),
    );
    text_area
}

fn mk_identity_text_area(s: &str) -> TextArea<'static> {
    let mut text_area: TextArea<'_> = TextArea::default();
    text_area.set_block(
        Block::default()
            .title("your identity (Enter to apply, Escape to cancel or use default, Ctrl-C to quit)")
            .borders(Borders::ALL),
    );
    text_area.insert_str(s);
    text_area
}

pub struct Identity {
    pub edit: Option<TextArea<'static>>,
    pub val: TextArea<'static>,
}

impl Default for Identity {
    fn default() -> Self {
        let me: String = "me".into();
        Self {
            edit: Some(mk_identity_text_area(&me)),
            val: mk_identity_text_area(&me),
        }
    }
}

pub async fn cmdline(
    mut cell: ActoCell<Cmdline, impl ActoRuntime>,
    display: ActoRef<Display>,
    mut messages: ActoRef<Messages>,
) {
    let identity = Writer::new(Identity::default());
    let text_area = Writer::new(mk_message_text_area());
    display.send(Display::UpdateIdentity(identity.reader()));
    display.send(Display::Cmdline(text_area.reader()));

    while let ActoInput::Message(msg) = cell.recv().await {
        match msg {
            Cmdline::Event(event) => {
                // Allow certain key to pass
                let key = match event {
                    // Prevent double input because of KeyEventKind::Release
                    Event::Key(key) if key.kind == KeyEventKind::Press || key.kind == KeyEventKind::Repeat => key,
                    _ => continue,
                };

                let is_editing_identity = { (identity.read()).edit.is_some() };
                match is_editing_identity {
                    // When editing identity
                    true => handle_key_on_identity_mode(&display, &identity, key),
                    // When chatting
                    false => handle_key_on_chat_mode(&display, &identity, &text_area, &messages, key),
                }
            }
            Cmdline::Reconnect(m) => messages = m,
        }
    }
}

fn handle_key_on_identity_mode(display: &ActoRef<Display>, identity: &Writer<Identity>, key: KeyEvent) {
    {
        let mut identity = identity.write();
        match key.code {
            KeyCode::Esc => {
                identity.edit = None;
            }
            KeyCode::Enter => {
                if let Some(identity_buffer_value) = identity.edit.take() {
                    identity.val = identity_buffer_value;
                }
            }
            _ => {
                if let Some(edit) = &mut identity.edit {
                    edit.input(key);
                }
            }
        }
    }
    display.send(Display::UpdateIdentity(identity.reader()));
}

fn handle_key_on_chat_mode(
    display: &ActoRef<Display>,
    identity: &Writer<Identity>,
    text_area: &Writer<TextArea<'static>>,
    messages: &ActoRef<Messages>,
    key: KeyEvent,
) {
    match key.code {
        KeyCode::PageDown => {
            display.send(Display::scroll_down(if key.modifiers.contains(KeyModifiers::CONTROL) {
                10
            } else {
                1
            }));
        }
        KeyCode::PageUp => {
            display.send(Display::scroll_up(if key.modifiers.contains(KeyModifiers::CONTROL) {
                10
            } else {
                1
            }));
        }
        KeyCode::Char('i') if key.modifiers.contains(KeyModifiers::CONTROL) => {
            {
                let mut identity = identity.write();
                identity.edit = Some(identity.val.clone());
            }
            display.send(Display::UpdateIdentity(identity.reader()));
        }
        _ => {
            // FIXME modifiers are not recognised, so no ctrl-enter
            if key.code == KeyCode::Enter {
                let mut text_area = text_area.write();
                let text = std::mem::replace(&mut *text_area, mk_message_text_area())
                    .into_lines()
                    .join("\n");
                tracing::info!("publishing message: {}", text);
                let name = {
                    let val = &identity.read().val;
                    let lines = val.clone().into_lines().join("\n");
                    lines
                };
                messages.send(Messages::Publish(messages::Event::new(name, text)));
            } else {
                text_area.write().input(key);
            }
            display.send(Display::Cmdline(text_area.reader()));
        }
    }
}