use anyhow::Result;
use node::run::{run, RunOpts};
use structopt::StructOpt;

pub fn main() -> Result<()> {
    run(RunOpts::from_args())
}
