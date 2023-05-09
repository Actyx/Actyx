use crate::util::spawn_with_name;
use crate::{formats::ShutdownReason, node_settings::Settings};
use anyhow::Result;
use crossbeam::{channel, select};
use derive_more::{Display, From};
use std::thread::JoinHandle;

pub mod android;
pub mod logging;
pub mod node_api;
pub mod store;
pub mod swarm_observer;

#[derive(Debug)]
pub enum ComponentRequest<A> {
    /// Component specific request
    Individual(A),
    /// Register a supervisor, which is informed about the component's state
    /// changes. Each Component is considered a singleton within the codebase.
    RegisterSupervisor(channel::Sender<(ComponentType, ComponentState)>),
    /// Global Settings have changed
    SettingsChanged(Box<Settings>),
    /// Trigger a stop and restart
    Restart,
    /// Trigger graceful shutdown
    Shutdown(ShutdownReason),
}

#[derive(Debug)]
pub enum ComponentState {
    Errored(anyhow::Error),
    Starting,
    Started,
    Stopped,
}

#[cfg(test)]
impl PartialEq for ComponentState {
    fn eq(&self, other: &Self) -> bool {
        std::mem::discriminant(self) == std::mem::discriminant(other)
    }
}

macro_rules! continue_on_error {
    ($c:expr, $l:expr) => {
        match $l {
            Err(e) => {
                tracing::error!("Component \"{}\": {}", $c, e);
                continue;
            }
            Ok(x) => x,
        }
    };
}

macro_rules! state_change {
    ($maybe_supervisor:expr, $c_name:expr, $target:expr, $result_transition:expr) => {
        let new_state = if let Err(e) = $result_transition {
            ComponentState::Errored(e)
        } else {
            $target
        };
        tracing::debug!("Component \"{}\": State change to {:?}", $c_name, new_state);
        match $maybe_supervisor.as_ref() {
            Some(snd) => {
                snd.send(($c_name.to_string().into(), new_state))?;
            }
            None => {
                tracing::error!("Component \"{}\": No supervisor registered.", $c_name)
            }
        }
    };
}

#[derive(Debug, Clone, Eq, PartialOrd, Ord, PartialEq, Display, From)]
pub struct ComponentType(String);

impl From<&str> for ComponentType {
    fn from(s: &str) -> Self {
        Self(s.to_owned())
    }
}

/// A `Component` is a self-contained package encapsulating a set of
/// functionality. This trait exposes defined ways to interact with the component
/// and manage its lifecycle. A component can provide individual `RequestType`s,
/// and provides distinct `ComponentSettings`. A standard implementation to drive
/// the component is provided in `Component::loop_on_rx`.
pub trait Component<RequestType, ComponentSettings>
where
    Self: Sized + Send + 'static,
    ComponentSettings: Clone + PartialEq,
    RequestType: std::fmt::Debug,
{
    /// Returns the type of the `Component`
    fn get_type() -> &'static str;

    /// Borrowed access to the held Receiver
    fn get_rx(&self) -> &channel::Receiver<ComponentRequest<RequestType>>;

    /// Handle a component specific request
    fn handle_request(&mut self, req: RequestType) -> Result<()>;

    /// Transform a complete `Settings` object into component specific
    /// `ComponentSettings`. In some cases a simple `Into` might not be
    /// sufficient, and access to self is necessary.
    fn extract_settings(&self, s: Settings) -> Result<(ComponentSettings, Vec<anyhow::Error>)>;

    /// New component specific `ComponentSettings`. Returned bool indicates
    /// whether a restart of the component is required.
    fn set_up(&mut self, _: ComponentSettings) -> bool {
        false
    }

    /// Start the component. This function should be idempotent and must be
    /// lock-free. Errors that happen during the startup or runtime of the
    /// component can be signalled using the provided `notifier`. Successful
    /// startup must be signalled by sending `Ok(())`.
    fn start(&mut self, notifier: channel::Sender<anyhow::Result<()>>) -> Result<()>;

    /// Stop the component. This function should be idempotent. It must be
    /// ensured, that all resources are cleaned up when returning from this
    /// method.
    fn stop(&mut self) -> Result<()>;

    /// Convenience implementation managing the lifecycle of a `Component` as
    /// driven by `ComponentRequest`s: New settings are converted to component
    /// specific ones; if they have been changed (as determined by Eq), the
    /// component is going to be stopped and started. This function will block
    /// and only return after receiving a `ComponentRequest::Shutdown` message.
    fn loop_on_rx(mut self) -> Result<()> {
        let mut last_settings: Option<ComponentSettings> = None;
        let mut supervisor: Option<channel::Sender<(ComponentType, ComponentState)>> = None;
        let (err_tx, err_rx) = channel::bounded::<anyhow::Result<()>>(8);
        let mut has_started = false;
        loop {
            select! {
                recv(err_rx) -> result => {
                    tracing::debug!("Component \"{}\": started", Self::get_type());
                    let result = result.expect("We keep another Sender around, thus channel can't be disconnected");
                    state_change!(
                        supervisor,
                        Self::get_type(),
                        ComponentState::Started,
                        result
                    );
                },
                recv(self.get_rx()) -> node_msg => {
                    if let Ok(m) = node_msg {
                        tracing::debug!("Component \"{}\": {:?}", Self::get_type(), m);
                        match m {
                            ComponentRequest::<RequestType>::Individual(m) => {
                                continue_on_error!(Self::get_type(), self.handle_request(m))
                            }
                            ComponentRequest::<RequestType>::RegisterSupervisor(snd) => {
                                if supervisor.replace(snd).is_some() {
                                    tracing::warn!("Component \"{}\": Supervisor registered twice!", Self::get_type());
                                } else {
                                    tracing::debug!("Component \"{}\": Supervisor registered", Self::get_type());
                                }
                            }
                            ComponentRequest::SettingsChanged(settings) => {
                                let (s, warnings) = continue_on_error!(Self::get_type(), self.extract_settings(*settings));
                                if warnings.len() > 0 {
                                    for warning in warnings {
                                        // warning is encoded in json_string to avoid new line in actyx log
                                        let warning_string = format!("{:?}", warning);
                                        if let Ok(warning_text_encoded_in_json_string) = serde_json::to_string(&warning_string){
                                            tracing::warn!("{:?}", warning_text_encoded_in_json_string);
                                        }
                                    }
                                }
                                let config_changed = !last_settings.iter().any(|c| *c == s);
                                if config_changed {
                                    tracing::debug!("Component \"{}\": Settings changed.", Self::get_type());
                                    let needs_restart =  self.set_up(s.clone());
                                    last_settings.replace(s);
                                    if !has_started || needs_restart {
                                        if has_started {
                                            state_change!(supervisor, Self::get_type(), ComponentState::Stopped, self.stop());
                                        }
                                        has_started = true;
                                        state_change!(
                                            supervisor,
                                            Self::get_type(),
                                            ComponentState::Starting,
                                            self.start(err_tx.clone())
                                        );
                                    }
                                }
                            }
                            ComponentRequest::Restart => {
                                if has_started {
                                    state_change!(supervisor, Self::get_type(), ComponentState::Stopped, self.stop());
                                }
                                has_started = true;
                                state_change!(
                                    supervisor,
                                    Self::get_type(),
                                    ComponentState::Starting,
                                    self.start(err_tx.clone())
                                );
                            }
                            ComponentRequest::<RequestType>::Shutdown(_) => break,
                        }

                    } else {
                        // Channel returned by `self.get_rx` is disconnected.
                        // Nothing else we can do but shut down
                        break;
                    }
                }
            }
            tracing::debug!("Component \"{}\": event handled", Self::get_type());
        }

        tracing::debug!("Component \"{}\": Shutting down", Self::get_type());
        state_change!(supervisor, Self::get_type(), ComponentState::Stopped, self.stop());
        Ok(())
    }

    /// Spawn the component into its own thread, where `Component::loop_on_rx` is
    /// executed. Returns a `std::thread::JoinHandle` to the spawned thread.
    fn spawn(self) -> Result<JoinHandle<()>> {
        let name = Self::get_type().to_string();
        let h = spawn_with_name(name.clone(), move || {
            if let Err(e) = self.loop_on_rx() {
                tracing::error!("Component \"{}\": Thread exited ({})", name, e);
            }
        });

        Ok(h)
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use anyhow::Result;
    use crossbeam::channel::Sender;
    use std::{
        sync::{Arc, Mutex},
        time::Duration,
    };

    struct SimpleComponent {
        rx: channel::Receiver<ComponentRequest<SimpleRequest>>,
        random_config: bool,
        last_cnt: usize,
        notifier: Arc<Mutex<Option<channel::Sender<Result<()>>>>>,
    }

    #[derive(Debug)]
    enum SimpleRequest {
        ToggleRandomConfigCreation,
        Ping(Sender<()>),
    }

    #[derive(Clone, PartialEq, Eq)]
    struct SimpleSettings {
        cnt: usize,
    }

    impl SimpleComponent {
        fn new(
            rx: channel::Receiver<ComponentRequest<SimpleRequest>>,
            notifier: Arc<Mutex<Option<channel::Sender<Result<()>>>>>,
        ) -> Self {
            Self {
                rx,
                random_config: false,
                last_cnt: 0,
                notifier,
            }
        }
    }
    impl Component<SimpleRequest, SimpleSettings> for SimpleComponent {
        fn get_type() -> &'static str {
            "test"
        }
        fn get_rx(&self) -> &channel::Receiver<ComponentRequest<SimpleRequest>> {
            &self.rx
        }
        fn set_up(&mut self, s: SimpleSettings) -> bool {
            self.last_cnt = s.cnt;
            true
        }
        fn handle_request(&mut self, x: SimpleRequest) -> Result<()> {
            match x {
                SimpleRequest::Ping(ponger) => ponger.send(()).unwrap(),
                SimpleRequest::ToggleRandomConfigCreation => {
                    self.random_config = !self.random_config;
                }
            }
            Ok(())
        }
        fn start(&mut self, notifier: Sender<Result<()>>) -> Result<()> {
            notifier.send(Ok(()))?;
            *self.notifier.lock().unwrap() = Some(notifier);
            Ok(())
        }
        fn stop(&mut self) -> Result<()> {
            let _ = self.notifier.lock().unwrap().take();
            Ok(())
        }
        fn extract_settings(&self, _: Settings) -> Result<(SimpleSettings, Vec<anyhow::Error>), anyhow::Error> {
            Ok((
                if self.random_config {
                    SimpleSettings { cnt: self.last_cnt + 1 }
                } else {
                    SimpleSettings { cnt: self.last_cnt }
                },
                vec![],
            ))
        }
    }

    #[test]
    fn lifecycle_shutdown() -> Result<()> {
        let (tx, rx) = channel::bounded(42);
        let c = SimpleComponent::new(rx, Default::default());
        let h = c.spawn()?;
        tx.send(ComponentRequest::Shutdown(ShutdownReason::TriggeredByHost))?;
        h.join().unwrap();
        Ok(())
    }

    const _3SEC: Duration = Duration::from_secs(3);

    #[test]
    fn setup_start_shutdown() -> Result<()> {
        let (tx, rx) = channel::bounded(42);
        let c = SimpleComponent::new(rx, Default::default());
        let h = c.spawn()?;
        let (tx_supervisor, rx_supervisor) = channel::bounded(42);
        tx.send(ComponentRequest::RegisterSupervisor(tx_supervisor))?;

        // Start on initial config
        tx.send(ComponentRequest::SettingsChanged(Box::new(Settings::sample())))?;
        assert_eq!(
            rx_supervisor.recv_timeout(_3SEC)?,
            ("test".into(), ComponentState::Starting)
        );
        assert_eq!(
            rx_supervisor.recv_timeout(_3SEC)?,
            ("test".into(), ComponentState::Started)
        );

        // Shutdown and yield
        tx.send(ComponentRequest::Shutdown(ShutdownReason::TriggeredByHost))?;
        assert_eq!(
            rx_supervisor.recv_timeout(_3SEC)?,
            ("test".into(), ComponentState::Stopped)
        );
        h.join().unwrap();
        Ok(())
    }

    #[test]
    fn setup_start_runtime_error() -> Result<()> {
        let (tx, rx) = channel::bounded(42);
        let err_notifier: Arc<Mutex<_>> = Default::default();
        let c = SimpleComponent::new(rx, err_notifier.clone());
        let h = c.spawn()?;
        let (tx_supervisor, rx_supervisor) = channel::bounded(42);
        tx.send(ComponentRequest::RegisterSupervisor(tx_supervisor))?;

        // Start on initial config
        tx.send(ComponentRequest::SettingsChanged(Box::new(Settings::sample())))?;
        assert_eq!(
            rx_supervisor.recv_timeout(_3SEC)?,
            ("test".into(), ComponentState::Starting)
        );
        assert_eq!(
            rx_supervisor.recv_timeout(_3SEC)?,
            ("test".into(), ComponentState::Started)
        );

        // trigger runtime error
        err_notifier
            .lock()
            .unwrap()
            .as_ref()
            .unwrap()
            .send(Err(anyhow::anyhow!("Sad cat is sad :-(")))
            .unwrap();
        if let (ComponentType(t), ComponentState::Errored { .. }) = rx_supervisor.recv_timeout(_3SEC)? {
            assert_eq!(t, "test");
        } else {
            panic!()
        };

        // Shutdown and yield
        tx.send(ComponentRequest::Shutdown(ShutdownReason::TriggeredByHost))?;
        assert_eq!(
            rx_supervisor.recv_timeout(_3SEC)?,
            ("test".into(), ComponentState::Stopped)
        );
        h.join().unwrap();
        Ok(())
    }

    #[test]
    fn setup_start_configchange_shutdown() -> Result<()> {
        let (tx, rx) = channel::bounded(42);
        let c = SimpleComponent::new(rx, Default::default());
        let h = c.spawn()?;
        let (tx_supervisor, rx_supervisor) = channel::bounded(42);
        tx.send(ComponentRequest::RegisterSupervisor(tx_supervisor))?;

        // Start on initial config
        tx.send(ComponentRequest::SettingsChanged(Box::new(Settings::sample())))?;
        assert_eq!(
            rx_supervisor.recv_timeout(_3SEC)?,
            ("test".into(), ComponentState::Starting)
        );
        assert_eq!(
            rx_supervisor.recv_timeout(_3SEC)?,
            ("test".into(), ComponentState::Started)
        );

        // Don't restart on unchanged config
        tx.send(ComponentRequest::SettingsChanged(Box::new(Settings::sample())))?;
        assert!(rx_supervisor.recv_timeout(Duration::from_secs(1)).is_err());

        // Restart on changed config
        tx.send(ComponentRequest::Individual(SimpleRequest::ToggleRandomConfigCreation))?;
        tx.send(ComponentRequest::SettingsChanged(Box::new(Settings::sample())))?;
        assert_eq!(
            rx_supervisor.recv_timeout(_3SEC)?,
            ("test".into(), ComponentState::Stopped)
        );
        assert_eq!(
            rx_supervisor.recv_timeout(_3SEC)?,
            ("test".into(), ComponentState::Starting)
        );
        assert_eq!(
            rx_supervisor.recv_timeout(_3SEC)?,
            ("test".into(), ComponentState::Started)
        );

        // Shutdown and yield
        tx.send(ComponentRequest::Shutdown(ShutdownReason::TriggeredByHost))?;
        assert_eq!(
            rx_supervisor.recv_timeout(_3SEC)?,
            ("test".into(), ComponentState::Stopped)
        );
        h.join().unwrap();
        Ok(())
    }

    #[test]
    fn respond_to_individual_request() -> Result<()> {
        let (tx, rx) = channel::bounded(42);
        let c = SimpleComponent::new(rx, Default::default());
        let h = c.spawn()?;
        let (pong_tx, pong_rx) = channel::bounded(1);

        tx.send(ComponentRequest::Individual(SimpleRequest::Ping(pong_tx)))?;
        assert!(pong_rx.recv_timeout(_3SEC).is_ok());
        assert!(pong_rx.try_recv().is_err());

        // Shutdown and yield
        tx.send(ComponentRequest::Shutdown(ShutdownReason::TriggeredByHost))?;
        h.join().unwrap();
        Ok(())
    }
}
