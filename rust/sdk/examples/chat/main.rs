use acto::{AcTokio, ActoCell, ActoInput, ActoRuntime};
use cmdline::Cmdline;
use display::Display;
use std::{env::var, fs::File, time::Duration};
use tracing_subscriber::EnvFilter;

mod cmdline;
mod display;
mod input;
mod messages;

fn main() {
    let logs = File::create("chat.log").expect("failed to create log file");
    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::new(var("RUST_LOG").unwrap_or_else(|_| "info".to_owned())))
        .with_writer(logs)
        .init();

    // reset terminal on panic
    let original_hook = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |panic| {
        display::reset_terminal();
        original_hook(panic);
    }));

    tracing::info!("starting runtime");
    let rt = AcTokio::new("chat", 1).expect("failed to create runtime");

    tracing::info!("starting actors");
    let handle = rt.spawn_actor("supervisor", supervisor).handle;

    tracing::info!("awaiting termination");
    rt.rt().block_on(handle.join()).unwrap();

    tracing::info!("terminating");
    rt.rt().block_on(rt.drop_done());
    tracing::info!("terminated");
}

enum Supervisor {
    Reconnect,
    Connected,
}

async fn supervisor(mut cell: ActoCell<Supervisor, impl ActoRuntime, anyhow::Result<()>>) {
    let display = cell.supervise(cell.spawn("display", display::display).map_handle(Ok));

    let me = cell.me();
    let mut messages = cell.spawn_supervised("messages", {
        let display = display.clone();
        move |cell| messages::messages(cell, me, display)
    });

    let cmdline = cell.supervise(
        cell.spawn("cmdline", {
            let display = display.clone();
            let messages = messages.clone();
            move |cell| cmdline::cmdline(cell, display, messages)
        })
        .map_handle(Ok),
    );

    cell.supervise(
        cell.spawn("input", {
            let cmdline = cmdline.clone();
            |cell| input::input(cell, cmdline)
        })
        .map_handle(Ok),
    );

    // wait for the first supervisor child to terminate
    loop {
        let i = cell.recv().await;
        match i {
            ActoInput::Supervision { id, name, result } => {
                tracing::info!("{} terminated with {:?}", name, result);
                if id == messages.id() {
                    let err = match result {
                        Ok(Err(e)) => e.to_string(),
                        Ok(Ok(_)) | Err(_) => break,
                    };
                    display.send(Display::NotConnected(err));
                    let me = cell.me();
                    tokio::spawn(async move {
                        tokio::time::sleep(Duration::from_millis(500)).await;
                        me.send(Supervisor::Reconnect);
                    });
                } else {
                    break;
                }
            }
            ActoInput::Message(Supervisor::Reconnect) => {
                let me = cell.me();
                let display = display.clone();
                messages = cell.spawn_supervised("messages", move |cell| messages::messages(cell, me, display));
                cmdline.send(Cmdline::Reconnect(messages.clone()));
            }
            ActoInput::Message(Supervisor::Connected) => {
                display.send(Display::Connected);
            }
            ActoInput::NoMoreSenders => {}
        }
    }
}
