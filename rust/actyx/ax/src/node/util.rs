use super::{formats::ExternalEvent, node_impl::NodeError, node_storage::NodeStorage, ApplicationState};
use crate::crypto::{KeyStore, KeyStoreRef};
use anyhow::{anyhow, Context};
use crossbeam::channel::Sender;
use parking_lot::RwLock;
use signal_hook::{consts::TERM_SIGNALS, low_level};
use std::{
    io,
    sync::{
        atomic::{AtomicU8, Ordering},
        Arc,
    },
    thread::Thread,
};

pub(crate) fn make_keystore(storage: NodeStorage) -> anyhow::Result<KeyStoreRef> {
    let ks = storage
        .get_keystore()?
        .map(|dump| {
            KeyStore::restore(io::Cursor::new(dump))
                .context(
                    "Error reading KeyStore (data corruption or invalid version)\n\n\
                    You may try to remove the `key_store` property from the `node` table in `actyx-data/node.sqlite`.",
                )
                .unwrap()
        })
        .unwrap_or_default();
    let ks = ks.with_cb(Box::new(move |vec| storage.dump_keystore(vec)));
    Ok(Arc::new(RwLock::new(ks)))
}

pub fn spawn_with_name<N, F, T>(name: N, f: F) -> std::thread::JoinHandle<T>
where
    F: FnOnce() -> T,
    F: Send + 'static,
    T: Send + 'static,
    N: Into<String>,
{
    std::thread::Builder::new()
        .name(name.into())
        .spawn(f)
        .expect("failed to spawn thread")
}

/// Install a global panic hook which is triggered by any panic in any thread
/// within this process. The panic incl its backtrace is logged, and `tx` is
/// notified. We could also just `process::exit` here, but it's highly unlikely
/// that the node's event loop is itself broken, so this provides a graceful way
/// to shutdown.
pub(crate) fn init_panic_hook(tx: Sender<ExternalEvent>) {
    std::panic::set_hook(Box::new(move |info| {
        // the backtrace library is the same lib that produces the dumps in std lib.
        let backtrace = backtrace::Backtrace::new();

        // formatting code inspired by the `log-panics` crate
        let thread = std::thread::current();
        let thread = thread.name().unwrap_or("unnamed");

        let err = if let Some(anyhow_err) = info.payload().downcast_ref::<Arc<anyhow::Error>>() {
            // Try to extract `NodeError` directly from `&Arc<anyhow::Error>`
            let err: NodeError = anyhow_err.into();
            err
        } else {
            let msg = match info.payload().downcast_ref::<&'static str>() {
                Some(s) => *s,
                None => match info.payload().downcast_ref::<String>() {
                    Some(s) => &**s,
                    None => "Box<Any>",
                },
            };

            let message = match info.location() {
                Some(location) => {
                    format!(
                        "thread '{}' panicked at '{}': {}:{}{:?}",
                        thread,
                        msg,
                        location.file(),
                        location.line(),
                        backtrace
                    )
                }
                None => format!("thread '{}' panicked at '{}'{:?}", thread, msg, backtrace),
            };
            tracing::error!(target: "panic", "{}", message);

            NodeError::InternalError(Arc::new(anyhow!(message)))
        };
        if tx
            .send(ExternalEvent::ShutdownRequested(
                super::formats::ShutdownReason::Internal(err),
            ))
            .is_err()
        {
            // Seems the node is not alive anymore, so exit here.
            std::process::exit(1)
        }
    }));
}

lazy_static::lazy_static! {
    static ref SHUTDOWN_FLAG: AtomicU8 = AtomicU8::new(0);
    static ref SHUTDOWN_THREAD: Thread = std::thread::current();
}

pub fn init_shutdown_ceremony() {
    SHUTDOWN_THREAD.unpark();
}

pub fn trigger_shutdown(success: bool) {
    let v = if success { 1 } else { 2 };
    SHUTDOWN_FLAG.store(v, Ordering::Release);
    SHUTDOWN_THREAD.unpark();
}

pub fn shutdown_ceremony(app_handle: ApplicationState) -> anyhow::Result<()> {
    for sig in TERM_SIGNALS {
        // if term_requested is already true, then this is the second signal, so exit
        unsafe {
            low_level::register(*sig, || {
                if SHUTDOWN_FLAG.load(Ordering::Acquire) > 0 {
                    low_level::exit(1);
                }
            })
        }
        .unwrap_or_else(|e| panic!("cannot register handler for signal {}: {}", sig, e));
        unsafe { low_level::register(*sig, || trigger_shutdown(true)) }
            .unwrap_or_else(|e| panic!("cannot register handler for signal {}: {}", sig, e));
    }

    // now the function of this thread is solely to keep the app_handle from dropping
    // until we actually want to trigger a graceful shutdown
    let mut ret;
    loop {
        ret = SHUTDOWN_FLAG.load(Ordering::Relaxed);
        if ret > 0 {
            break;
        }
        std::thread::park();
        tracing::trace!("wake-up of guardian thread");
    }
    tracing::debug!(flag = ret, "graceful shutdown triggered");
    drop(app_handle);

    if ret == 1 {
        Ok(())
    } else {
        anyhow::bail!("stopped due to a failure in another thread")
    }
}
