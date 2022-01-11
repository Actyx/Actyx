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
    pub fn new(level: LogSeverity) -> Self {
        // If the `RUST_LOG` env var is set, the filter is statically set to
        // said value. This supports the common RUST_LOG syntax, see
        // https://docs.rs/tracing-subscriber/0.2.17/tracing_subscriber/fmt/index.html#filtering-events-with-environment-variables
        // Any overrides via `ax settings` will be ignored
        let (filter, level_from_env) = if let Ok(filter) = EnvFilter::try_from_default_env() {
            (filter, true)
        } else {
            (EnvFilter::new(level.to_string()), false)
        };
        let builder = tracing_subscriber::FmtSubscriber::builder()
            .with_span_events(FmtSpan::ENTER | FmtSpan::CLOSE)
            .with_env_filter(filter)
            .with_writer(std::io::stderr)
            .with_filter_reloading();
        // Store a handle to the generated filter (layer), so it can be swapped later
        let filter_handle = Box::new(builder.reload_handle()) as Box<dyn ReloadHandle + Send>;
        let subscriber = builder.finish();
        #[cfg(windows)]
        // Add additional layer on Windows, so the logs also end up in the
        // Windows event log
        let subscriber = {
            use tracing_subscriber::layer::SubscriberExt;
            subscriber.with(tracing_win_event_log::layer("Actyx").unwrap())
        };
        #[cfg(target_os = "android")]
        // Add additional layer on Android, so the logs also end up in logcat
        let subscriber = {
            use tracing_subscriber::layer::SubscriberExt;
            subscriber.with(tracing_android::layer("com.actyx").unwrap())
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
                    level,
                    DEFAULT_ENV,
                    std::env::var(DEFAULT_ENV).unwrap_or_else(|_| "".into())
                );
            } else {
                let new_filter = EnvFilter::new(level.to_string());
                if let Err(e) = self.filter_handle.reload(new_filter) {
                    eprintln!("Error installing new EnvFilter with severity {}: {}", level, e);
                    tracing::error!("Error installing new EnvFilter with severity {}: {}", level, e);
                }
            }
        }
        Ok(())
    }
}
