use anyhow::Result;
use app_agent::AppAgent;
use crossterm::{
    event::{self, Event, KeyCode},
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
    ExecutableCommand,
};
use ratatui::{prelude::*, widgets::*};
use std::io::{stdout, Write};
use tokio::join;
mod app_agent;

#[tokio::main]
async fn main() -> Result<()> {
    enable_raw_mode()?;
    stdout().execute(EnterAlternateScreen)?;

    let (app_agent_join_handle, app_agent) = app_agent::init();

    let ui_join_handle = tokio::spawn(ui_agent(app_agent));

    let (_, _) = join!(ui_join_handle, app_agent_join_handle);

    disable_raw_mode()?;
    stdout().execute(LeaveAlternateScreen)?;
    Ok(())
}

async fn ui_agent(app_agent: AppAgent) -> Result<()> {
    let mut terminal = Terminal::new(CrosstermBackend::new(stdout()))?;

    let mut should_quit = false;
    while !should_quit {
        display(&mut terminal, &app_agent).await;
        should_quit = handle_events(&app_agent).await?;
        tokio::task::yield_now().await;
    }

    Ok(())
}

async fn handle_events(app_agent: &AppAgent) -> Result<bool> {
    if event::poll(std::time::Duration::from_millis(50))? {
        if let Event::Key(key) = event::read()? {
            if key.kind == event::KeyEventKind::Press && key.code == KeyCode::Char('q') {
                print!("killing");
                app_agent.kill().await;
                print!("killing2");
                return Ok(true);
            }
        }
    }
    Ok(false)
}

async fn display<T: Write>(terminal: &mut Terminal<CrosstermBackend<T>>, app_agent: &AppAgent) {
    let _ = terminal.draw(|frame| {
        let main_layout = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(1),
                Constraint::Min(0),
                Constraint::Length(1),
            ])
            .split(frame.size());

        frame.render_widget(
            Block::new()
                .borders(Borders::TOP)
                .title(format!("Identity: {}", &app_agent.identity)),
            main_layout[0],
        );
        frame.render_widget(
            Block::new().borders(Borders::TOP).title("Title Bar"),
            main_layout[2],
        );
    });
}
