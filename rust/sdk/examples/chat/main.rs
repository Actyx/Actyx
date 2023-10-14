use acto::{AcTokio, ActoCell, ActoHandle, ActoInput, ActoRuntime};
use std::{env::var, fs::File, future::poll_fn, time::Duration};
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

    tracing::info!("starting runtime");
    let rt = AcTokio::new("chat", 1).expect("failed to create runtime");

    tracing::info!("starting actors");
    let mut handle = rt.spawn_actor("supervisor", supervisor).handle;

    tracing::info!("awaiting termination");
    rt.rt().block_on(poll_fn(move |cx| handle.poll(cx))).unwrap();

    tracing::info!("terminating");
    // FIXME: need to figure out how to wait for AcTokio task drops to have finished
    std::thread::sleep(Duration::from_millis(300));
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
