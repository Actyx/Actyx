use accessory::Context;

fn main() {
    tracing_subscriber::fmt::init();

    let ctx = Context::open().unwrap();
    loop {
        match ctx.block_read_uid(false) {
            Err(e) => println!("Error reading card {}", e),
            Ok(uid) => println!("Card UID {:X?}", uid),
        }
    }
}
