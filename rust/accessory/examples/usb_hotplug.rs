use accessory::HotPlugHandler;
use crossbeam::channel::{unbounded, TryRecvError};
use rusb::UsbContext;
use std::time::Duration;

fn main() {
    let context = rusb::Context::new().unwrap();
    let (tx, rx) = unbounded();
    let _handler = HotPlugHandler::try_setup(&context, tx);
    loop {
        context.handle_events(Some(Duration::from_millis(50))).unwrap();
        match rx.try_recv() {
            Ok(d) => println!("{:?}", d),
            Err(TryRecvError::Empty) => {}
            e => panic!(e),
        }
    }
}
