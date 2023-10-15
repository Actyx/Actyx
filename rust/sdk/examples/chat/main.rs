use acto::{AcTokio, ActoCell, ActoInput, ActoRuntime};
use std::{env::var, fs::File};
use tracing_subscriber::EnvFilter;
use void::Void;

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

async fn supervisor(mut cell: ActoCell<Void, impl ActoRuntime>) {
    let display = cell.spawn_supervised("display", display::display);
    let display2 = display.clone();
    let messages = cell.spawn_supervised("messages", move |cell| messages::messages(cell, display));
    let cmdline = cell.spawn_supervised("cmdline", move |cell| cmdline::cmdline(cell, display2, messages));
    cell.spawn_supervised("input", |cell| input::input(cell, cmdline));

    // wait for the first supervisor child to terminate
    while let ActoInput::NoMoreSenders = cell.recv().await {}
}
