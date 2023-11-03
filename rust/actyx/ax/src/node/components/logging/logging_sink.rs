use tracing::Subscriber;
use tracing_subscriber::{fmt::format::FmtSpan, layer::Layer, reload, reload::Handle, EnvFilter};

use util::formats::{ActyxOSResult, LogSeverity};

// Wrapper trait to contain the types
trait ReloadHandle {
    fn reload(&self, new_filter: EnvFilter) -> Result<(), reload::Error>;
}

const DEFAULT_ENV: &str = "RUST_LOG";
impl<L, S> ReloadHandle for Handle<L, S>
where
    L: From<EnvFilter> + Layer<S> + 'static,
    S: Subscriber,
{
    fn reload(&self, new_filter: EnvFilter) -> Result<(), reload::Error> {
        self.reload(new_filter)
    }
}
/// Convenience struct wrapping access to the installed log interceptor.
pub struct LoggingSink {
    configured_level: LogSeverity,
    level_from_env: bool,
    filter_handle: Box<dyn ReloadHandle + Send>,
}

impl LoggingSink {
    pub fn new(level: LogSeverity, log_no_color: bool, log_as_json: bool) -> Self {
        // If the `RUST_LOG` env var is set, the filter is statically set to
        // said value. This supports the common RUST_LOG syntax, see
        // https://docs.rs/tracing-subscriber/0.2.17/tracing_subscriber/fmt/index.html#filtering-events-with-environment-variables
        // Any overrides via `ax settings` will be ignored
        let (filter, level_from_env) = match EnvFilter::try_from_default_env() {
            Ok(filter) => (filter, true),
            Err(e) => {
                if std::env::var(EnvFilter::DEFAULT_ENV).is_ok() {
                    eprintln!("tracing: falling back to {}, error parsing RUST_LOG: {}", level, e);
                }
                (EnvFilter::new(level.to_string()), false)
            }
        };
        let log_color = !log_no_color;

        let builder = tracing_subscriber::FmtSubscriber::builder().with_span_events(FmtSpan::ENTER | FmtSpan::CLOSE);
        // Store a handle to the generated filter (layer), so it can be swapped later
        let (subscriber, filter_handle): (
            Box<dyn Subscriber + Sync + Send + 'static>,
            Box<dyn ReloadHandle + Send>,
        ) = if log_as_json {
            let builder = builder
                .json()
                .flatten_event(true)
                .with_env_filter(filter)
                .with_ansi(log_color)
                .with_writer(std::io::stderr)
                .with_filter_reloading();
            let filter_handle = Box::new(builder.reload_handle());
            let subscriber = builder.finish();
            #[cfg(target_os = "android")]
            let subscriber = tracing_android::layer("com.actyx").unwrap().with_subscriber(subscriber);
            let sub = Box::new(subscriber);
            (sub, filter_handle)
        } else {
            let builder = builder
                .with_env_filter(filter)
                .with_ansi(log_color)
                .with_writer(std::io::stderr)
                .with_filter_reloading();
            let filter_handle = Box::new(builder.reload_handle());
            let subscriber = builder.finish();
            #[cfg(target_os = "android")]
            let subscriber = tracing_android::layer("com.actyx").unwrap().with_subscriber(subscriber);
            let sub = Box::new(subscriber);
            (sub, filter_handle)
        };
        if tracing::subscriber::set_global_default(subscriber).is_err() {
            eprintln!("`tracing::subscriber::set_global_default` has been called more than once!");
            tracing::error!("`tracing::subscriber::set_global_default` has been called more than once!");
        }
        Self {
            configured_level: level,
            level_from_env,
            filter_handle,
        }
    }

    pub fn set_level(&mut self, level: LogSeverity) -> ActyxOSResult<()> {
        if self.configured_level != level {
            // Still store the configured level, so that we don't spam the logs
            self.configured_level = level;
            if self.level_from_env {
                tracing::info!(
                    "Ignoring set log level \"{}\", as the log filter is set via the \"{}\" environment variable (\"{}\")",
                    self.configured_level,
                    DEFAULT_ENV,
                    std::env::var(DEFAULT_ENV).unwrap_or_else(|_| "".into())
                );
            } else {
                let new_filter = EnvFilter::new(self.configured_level.to_string());
                if let Err(e) = self.filter_handle.reload(new_filter) {
                    eprintln!(
                        "Error installing new EnvFilter with severity {}: {}",
                        self.configured_level, e
                    );
                    tracing::error!(
                        "Error installing new EnvFilter with severity {}: {}",
                        self.configured_level,
                        e
                    );
                }
            }
        }
        Ok(())
    }
}
