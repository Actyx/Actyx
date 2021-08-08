// Hide command prompt
#![windows_subsystem = "windows"]

#[cfg(not(windows))]
fn main() {
    panic!("This program is only intended to run on Windows. Maybe you were looking for \"actyx-linux\"?");
}

#[cfg(windows)]
fn main() -> Result<(), anyhow::Error> {
    use structopt::StructOpt;
    let opts = win::Opts::from_args();
    let foreground = !opts.background;
    if let Err(e) = win::run(opts) {
        eprintln!("Actyx returned with an error: {:?}", e);
        if foreground {
            message_box::create("Actyx stopped", &*format!("{:?}", e))?;
        }
        Err(e)
    } else {
        Ok(())
    }
}

#[cfg(windows)]
mod message_box;
#[cfg(windows)]
mod win;
