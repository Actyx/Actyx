use crate::{
    display::Display,
    messages::{self, Messages},
};
use acto::{variable::Writer, ActoCell, ActoInput, ActoRef, ActoRuntime};
use crossterm::event::{Event, KeyCode};
use ratatui::widgets::{Block, Borders};
use tui_textarea::TextArea;

fn mk_text_area() -> TextArea<'static> {
    let mut text_area = TextArea::default();
    text_area.set_block(
        Block::default()
            .title("your message (Enter to send, Ctrl-C to quit)")
            .borders(Borders::ALL),
    );
    text_area
}

pub async fn cmdline(
    mut cell: ActoCell<Event, impl ActoRuntime>,
    display: ActoRef<Display>,
    messages: ActoRef<Messages>,
) {
    let text_area = Writer::new(mk_text_area());
    display.send(Display::Cmdline(text_area.reader()));

    while let ActoInput::Message(event) = cell.recv().await {
        if let Event::Key(key) = event {
            // FIXME modifiers are not recognised, so no ctrl-enter
            if key.code == KeyCode::Enter {
                let mut text_area = text_area.write();
                let text = std::mem::replace(&mut *text_area, mk_text_area())
                    .into_lines()
                    .join("\n");
                tracing::info!("publishing message: {}", text);
                messages.send(Messages::Publish(messages::Event::new("me".to_owned(), text)));
            } else {
                text_area.write().input(key);
            }
            display.send(Display::Cmdline(text_area.reader()));
        }
    }
}
