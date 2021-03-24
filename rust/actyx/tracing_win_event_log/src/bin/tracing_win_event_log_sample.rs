#[cfg(windows)]
mod sample_lib {
    use tracing::{debug, field, span, Level};

    pub struct Config;

    impl Config {
        pub fn load_global_config() {
            let question = "the answer to the ultimate question of life, the universe, and everything";
            let answer = 42;
            debug!(
                question.answer = answer,
                question.tricky = true,
                "the answer to {} is {}.",
                question,
                answer
            );

            let span = span!(Level::TRACE, "my span", bar = field::Empty);
            let entered_span = span.enter();
            debug!("is this in a span?");
            debug!("is this in a span 2?");
            drop(entered_span);
            debug!("this should not be in a span");
        }
    }
}

#[cfg(windows)]
fn main() {
    use sample_lib::Config;
    use tracing::error;
    use tracing_subscriber::{fmt, prelude::__tracing_subscriber_SubscriberExt, registry, util::SubscriberInitExt};
    let registry = registry();
    //tracing_win_event_log::try_register_log_source_in_win_registry("ActyxOS").unwrap();
    match tracing_win_event_log::layer("ActyxFoo") {
        Ok(layer) => {
            registry.with(layer).init();
        }
        Err(e) => {
            registry.with(fmt::layer().json()).init();
            error!("Couldn't connect to Windows event log: {}", e);
        }
    }

    error!("Sample app v{}", env!("CARGO_PKG_VERSION"));
    Config::load_global_config();
}

#[cfg(not(windows))]
fn main() {}
