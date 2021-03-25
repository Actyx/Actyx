// Hide command prompt
#![windows_subsystem = "windows"]

#[cfg(not(windows))]
fn main() {
    panic!("This program is only intended to run on Windows. Maybe you were looking for \"actyx-linux\"?");
}

#[cfg(windows)]
fn main() -> Result<(), anyhow::Error> {
    if let Err(e) = win::run() {
        message_box::create("Actyx stopped", &*format!("{}", e))?
    }
    Ok(())
}

#[cfg(windows)]
mod message_box;
#[cfg(windows)]
mod win {
    use crossbeam::channel::{RecvTimeoutError, TryRecvError};
    use node::{BindTo, BindToOpts};
    use std::{
        path::PathBuf,
        process::Child,
        sync::{
            atomic::{AtomicBool, Ordering},
            Arc, Mutex,
        },
        time::Duration,
    };
    use structopt::StructOpt;
    use tracing::*;

    #[derive(StructOpt, Debug)]
    #[structopt(name = "actyx", about = "Actyx on Windows", rename_all = "kebab-case")]
    struct Opts {
        #[structopt(long, env = "ACTYX_PATH")]
        /// Path where to store all the data of the Actyx node
        /// defaults to creating <current working dir>/actyx-data
        working_dir: Option<PathBuf>,

        /// Hides the system tray icon
        #[structopt(long = "background")]
        background: bool,

        #[structopt(flatten)]
        bind_options: BindToOpts,
    }

    struct TrayApp {
        tray: systray::Application,
        nodemanager_handle: Arc<Mutex<Option<Child>>>,
    }

    impl TrayApp {
        fn try_new() -> anyhow::Result<Self> {
            let mut trayicon_app = systray::Application::new()?;
            // This assumes, that Actyx on Windows has been installed properly with
            // our provided installer.
            let executable_dir = std::env::current_exe()?;

            // The icon is added as a resource with a default identifier 1 in
            // `build.rs`. In order to load it, one needs to prefix it with #
            // (https://docs.microsoft.com/en-us/windows/win32/api/winuser/nf-winuser-loadimagew#remarks)
            trayicon_app.set_icon_from_resource("#1")?;
            trayicon_app.set_tooltip("Actyx")?;

            let maybe_node_manager = executable_dir.join("../node-manager/actyxos-node-manager.exe");
            let nodemanager_handle = Arc::new(Mutex::new(Option::<Child>::None));
            let handle_c = nodemanager_handle.clone();
            if maybe_node_manager.exists() {
                trayicon_app.add_menu_item("Launch Node Manager", move |_| {
                    let mut guardian = handle_c.lock().unwrap();
                    let is_running = (*guardian)
                        .as_mut()
                        // Returns Ok(None) if running
                        .map(|c| c.try_wait().unwrap().is_none())
                        .unwrap_or(false);
                    if is_running {
                        info!("Node Manager already running.");
                    } else {
                        match std::process::Command::new(maybe_node_manager.clone()).spawn() {
                            Ok(child) => {
                                *guardian = Some(child);
                            }
                            Err(e) => {
                                error!("Error spawning Node Manager: {}", e);
                            }
                        }
                    }
                    Ok::<_, systray::Error>(())
                })?;
                trayicon_app.add_menu_separator()?;
            }

            trayicon_app.add_menu_item("Exit", |window| {
                // This will result in `app.wait_for_message` returning at the
                // end of this function, which may end the `tray.try_wait` in
                // `Self::drive` below
                window.quit();
                Ok::<_, systray::Error>(())
            })?;

            Ok(TrayApp {
                tray: trayicon_app,
                nodemanager_handle,
            })
        }
        fn drive(&mut self, timeout: Duration) -> Result<(), systray::Error> {
            // Blocks until timeout passed, or an error occured, or
            // `app.quit`/`window.quit` is called
            self.tray.try_wait(timeout)?;
            Ok(())
        }
    }

    impl Drop for TrayApp {
        fn drop(&mut self) {
            // Stop node_manager, if started
            if let Some(mut child) = self.nodemanager_handle.lock().unwrap().take() {
                let _ = child.kill().and_then(|_| child.wait()).map_err(|e| {
                    error!("Error stopping Node Manager: {}. The Actyx process might not shut down properly because of a dangling child.", e);
                    e
                });
            }
        }
    }

    pub(crate) fn run() -> Result<(), anyhow::Error> {
        use crossbeam::channel::{bounded, select, tick};
        use node::{ApplicationState, Runtime};
        use std::convert::TryInto;
        // Make sure, there's only one instance of Actyx running on the system.
        // On Windows this is implemented by creating named mutex with CreateMutexW.
        // On UNIX systems this is implemented by using files and flock. The path of the
        // created lock file will be /tmp/<name>.lock.
        // The user won't be notified on Windows about this, as this application is
        // running without a console. Not much we can do about this at this point.
        let global_mutex = named_lock::NamedLock::create("Actyx")
            .map_err(|e| anyhow::anyhow!("Error creating global mutex: {}", e))?;
        let _global_guard = global_mutex.try_lock().map_err(|_| {
            anyhow::anyhow!("Error acquiring global mutex. Maybe another Actyx instance is already running?")
        })?;
        let Opts {
            working_dir: maybe_working_dir,
            background,
            bind_options,
        } = Opts::from_args();
        let bind_to: BindTo = bind_options.try_into()?;

        let working_dir = maybe_working_dir.unwrap_or_else(|| std::env::current_dir().unwrap().join("actyx-data"));
        std::fs::create_dir_all(working_dir.clone())
            .map_err(|e| anyhow::anyhow!("Unable to create working directory ({}): {}", working_dir.display(), e))?;

        // Spawn Actyx as fast as possible, so the logging infrastructure is
        // set up.
        let mut app_handle = ApplicationState::spawn(working_dir, Runtime::Windows, bind_to)?;
        // Receiver from node
        let result_recv = app_handle.manager.rx_process.take().unwrap();

        // If running in foreground (e.g. with tray), use this atomic bool to
        // signal shutdown
        let running = Arc::new(AtomicBool::new(true));
        let running2 = running.clone();
        wintrap::trap(
            &[
                wintrap::Signal::CtrlC,
                wintrap::Signal::CloseWindow,
                wintrap::Signal::CtrlBreak,
                wintrap::Signal::CloseConsole,
            ],
            move |signal| {
                // handle signal here
                info!("Caught a signal: {:?}. Shutting down.", signal);
                running.store(false, Ordering::SeqCst);
            },
            || -> Result<(), anyhow::Error> {
                if background {
                    while running2.load(Ordering::SeqCst) {
                        match result_recv.recv_timeout(Duration::from_millis(500)) {
                            Ok(node_yielded) => return node_yielded.map_err(Into::into),
                            Err(RecvTimeoutError::Disconnected) => {
                                return Err(anyhow::anyhow!("Actyx node shut down without yielding an exit code."))
                            }
                            _ => {}
                        }
                    }
                } else {
                    let mut tray = TrayApp::try_new()?;
                    while running2.load(Ordering::SeqCst) {
                        match tray.drive(Duration::from_millis(500)) {
                            Err(systray::Error::TimeoutError) => {}
                            Err(e) => {
                                error!("Error setting up Windows GUI: {}", e);
                                return Err(e.into());
                            }
                            Ok(_) => {
                                info!("Shutdown via GUI");
                                return Ok(());
                            }
                        }
                        match result_recv.try_recv() {
                            Ok(node_yielded) => {
                                return node_yielded.map_err(Into::into);
                            }
                            Err(TryRecvError::Disconnected) => {
                                return Err(anyhow::anyhow!("Node vanished. Look for other logs."))
                            }
                            _ => {}
                        }
                    }
                }
                Ok(())
            },
        )??;

        // Stopping Actyx is sometimes tricky business; if it wasn't dropped
        // within a couple of seconds, forcefully exit.
        let (tx, rx) = bounded(1);
        // Offload shutdown to another thread
        let drop_handle = std::thread::spawn(move || {
            drop(app_handle);
            tx.send(()).unwrap();
        });
        loop {
            select! {
                recv(tick(Duration::from_secs(5))) -> _ => {
                    eprintln!("Actyx didn't close within 5 secs. Exiting ..");
                    std::process::exit(1);
                },
                recv(rx) -> _ => {
                    // Shutdown finished
                    drop_handle.join().unwrap();
                    break;
                },
            }
        }
        Ok(())
    }
}
