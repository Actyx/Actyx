use crate::{formats::ExternalEvent, node::NodeError, node_storage::NodeStorage};
use anyhow::{anyhow, Context};
use crossbeam::channel::Sender;
use crypto::{KeyStore, KeyStoreRef};
use parking_lot::RwLock;
use std::{io, sync::Arc};

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
                crate::formats::ShutdownReason::Internal(err),
            ))
            .is_err()
        {
            // Seems the node is not alive anymore, so exit here.
            std::process::exit(1)
        }
    }));
}

pub(crate) fn env_var_is_truish(var: &str) -> Option<bool> {
    match std::env::var(var).map(|s| s.to_lowercase()).as_deref() {
        Err(_) => None,
        Ok("auto") => Some(true),
        Ok("on") => Some(true),
        Ok("true") => Some(true),
        Ok("1") => Some(true),
        Ok("always") => Some(true),
        Ok("off") => Some(false),
        Ok("false") => Some(false),
        Ok("0") => Some(false),
        Ok("never") => Some(false),
        Ok(_) => None,
    }
}

pub mod shutdown {
    use super::spawn_with_name;
    use crate::ApplicationState;
    use crossbeam::channel::{bounded, select};
    use signal_hook::consts::TERM_SIGNALS;
    use signal_hook::flag;
    use std::sync::atomic::{AtomicBool, Ordering};
    use std::sync::Arc;
    pub fn shutdown_ceremony(mut app_handle: ApplicationState) {
        let (signal_tx, signal_rx) = bounded::<()>(8);

        spawn_with_name("SignalHandling", move || {
            let term_requested = Arc::new(AtomicBool::new(false));
            let immediate_term_requested = Arc::new(AtomicBool::new(false));
            for sig in TERM_SIGNALS {
                flag::register(*sig, Arc::clone(&term_requested))
                    .expect("registered for termination signals (SIGTERM, SIGINT)");
            }
            let mut requested = false;
            let mut immediate_requested = false;
            loop {
                if term_requested.load(Ordering::Relaxed) && !requested {
                    requested = true;
                    tracing::trace!("caught termination signal for first time");
                    for sig in TERM_SIGNALS {
                        flag::register(*sig, Arc::clone(&immediate_term_requested))
                            .expect("registered for termination signals (SIGTERM, SIGINT)");
                    }
                    signal_tx.send(()).unwrap();
                }
                if immediate_term_requested.load(Ordering::Relaxed) && !immediate_requested {
                    immediate_requested = true;
                    eprintln!("caught second termination signal; force exiting...");
                    tracing::trace!("caught termination signal for second time");
                    signal_tx.send(()).unwrap();
                }
                if requested && immediate_requested {
                    return;
                }
            }
        });
        let mut sig_count = 0;
        let mut handle = None;
        let (tx, rx) = bounded(1);
        let result_recv = app_handle.manager.rx_process.take().unwrap();
        let mut app_container = Some((app_handle, tx));
        loop {
            select! {
                recv(signal_rx) -> _ => {
                    tracing::trace!("received termination signal; count={}", sig_count);
                    match sig_count {
                        0 => {
                            tracing::debug!("termination signal received once; requesting gracefull node shutdown...");
                            // Offload shutdown to another thread
                            let (app_handle, tx) = app_container.take().unwrap();
                            handle = Some(std::thread::spawn(move || {
                                drop(app_handle);
                                tx.send(()).unwrap();
                            }));
                        },
                        _ => {
                            tracing::warn!("termination signal received twice; forecfully shutting down node...");
                            std::process::exit(1);
                        }
                    }
                    sig_count += 1;
                },
                recv(rx) -> _ => {
                    // Graceful shutdown finished
                    handle.unwrap().join().unwrap();
                    tracing::debug!("graceful shutdown has finished");
                    return;
                },
                recv(result_recv) -> res => {
                    // If the process is being terminated because of a signal, ignore
                    if sig_count == 0 {
                        if let Err(e) = res {
                            eprintln!("Node exited: {}", e);
                        }
                        return;
                    }
                }
            }
        }
    }
}
