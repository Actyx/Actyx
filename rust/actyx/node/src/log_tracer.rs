use tracing_log::AsTrace;

pub struct LogTracer {
    ignore: Vec<String>,
    log: tracing_log::LogTracer,
}

impl LogTracer {
    pub fn new<I: Into<String>>(ignore: impl IntoIterator<Item = I>) -> Self {
        let ignore = ignore.into_iter().map(|x| x.into()).collect();
        let log = tracing_log::LogTracer::default();
        Self { ignore, log }
    }
}

impl log::Log for LogTracer {
    fn enabled(&self, metadata: &log::Metadata<'_>) -> bool {
        // First, check the log record against the current max level enabled by
        // the current `tracing` subscriber.
        if metadata.level().as_trace() > tracing::level_filters::LevelFilter::current() {
            // If the log record's level is above that, disable it.
            return false;
        }

        // Okay, it wasn't disabled by the max level — do we have any specific
        // modules to ignore?
        if !self.ignore.is_empty() {
            // If we are ignoring certain module paths, ensure that the metadata
            // does not start with one of those paths.
            let target = metadata.target();
            for ignored in &self.ignore[..] {
                if target.starts_with(ignored) {
                    return metadata.level() < log::Level::Debug;
                }
            }
        }

        // Finally, check if the current `tracing` dispatcher cares about this.
        tracing::dispatcher::get_default(|dispatch| dispatch.enabled(&metadata.as_trace()))
    }

    fn log(&self, record: &log::Record<'_>) {
        // those funny `log` crate idiots don’t actually call `enabled` before logging ...
        if self.enabled(record.metadata()) {
            // we can’t log the record ourselves because there is some recursive magic between
            // tracing-subscriber and tracing-log that makes logging the right target work
            self.log.log(record);
        }
    }

    fn flush(&self) {}
}
