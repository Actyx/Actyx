use actyxos_lib::{ActyxOSResult, LogRequest, LogSeverity};
use crossbeam::channel::Sender;
use tracing::Subscriber;
use tracing_subscriber::{layer::Layer, reload, reload::Handle, EnvFilter};
use trees::wrapping_subscriber::WrappingSubscriber2;

// Wrapper trait to contain the types
trait ReloadHandle {
    fn reload(&self, new_filter: EnvFilter) -> Result<(), reload::Error>;
}

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
    filter_handle: Box<dyn ReloadHandle + Send>,
}
impl LoggingSink {
    pub fn new(level: LogSeverity, log_tx: Sender<LogRequest>) -> Self {
        let filter = EnvFilter::new(level.to_string());
        let builder = tracing_subscriber::FmtSubscriber::builder()
            .with_env_filter(filter)
            .with_filter_reloading();
        // Store a handle to the generated filter (layer), so it can be swapped later
        let filter_handle = Box::new(builder.reload_handle()) as Box<dyn ReloadHandle + Send>;
        let subscriber = builder.finish();
        #[cfg(windows)]
        // Add additional layer on Windows, so the logs also end up in the
        // Windows event log
        let subscriber = {
            use tracing_subscriber::layer::SubscriberExt;
            subscriber.with(tracing_win_event_log::layer("ActyxOS").unwrap())
        };
        // Ignore this crates' logs (deadly loops are lurking here ..)
        let subscriber = WrappingSubscriber2::new(subscriber, log_tx, env!("CARGO_PKG_NAME").to_string());
        if tracing::subscriber::set_global_default(subscriber).is_err() {
            eprintln!("`tracing::subscriber::set_global_default` has been called more than once!");
            tracing::error!("`tracing::subscriber::set_global_default` has been called more than once!");
        }
        Self {
            configured_level: level,
            filter_handle,
        }
    }
    pub fn set_level(&mut self, level: LogSeverity) -> ActyxOSResult<()> {
        if self.configured_level != level {
            self.configured_level = level;
            let new_filter = EnvFilter::new(level.to_string());
            if let Err(e) = self.filter_handle.reload(new_filter) {
                eprintln!("Error installing new EnvFilter with severity {}: {}", level, e);
                tracing::error!("Error installing new EnvFilter with severity {}: {}", level, e);
            }
        }
        Ok(())
    }
}
