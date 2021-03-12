#![deny(clippy::future_not_send)]

pub mod chunkedresponse;
pub mod formats;

pub use formats::*;

use std::io;
use tracing_subscriber::EnvFilter;

pub fn set_log_level(verbosity: u64) -> u64 {
    let log_string = if verbosity == 0 {
        // Special case for no verbosity on the command line
        // Try to read from the default environment variable or fall back to error
        // The format of the `RUST_LOG` variable is described in depth here
        // https://docs.rs/env_logger/latest/env_logger/#enabling-logging
        std::env::var("RUST_LOG").unwrap_or_else(|_| "error".to_string())
    } else {
        // A bit of stderrlog compatibility
        // Add log level from the v:s starting with 1 as warning
        match verbosity {
            1 => "warning",
            2 => "info",
            3 => "debug",
            _ => "trace",
        }
        .to_string()
    };
    let filter = EnvFilter::new(log_string);
    let subscriber = tracing_subscriber::FmtSubscriber::builder()
        .with_env_filter(filter)
        // By default, it outputs to stdout.
        .with_writer(io::stderr)
        .finish();
    tracing::subscriber::set_global_default(subscriber).unwrap();
    verbosity
}
