/// A prototype of additional ax-core/ax-sdk service
pub use anyhow::Context;
use ax_core::api::EventService;
pub use ax_core::node::BindTo;
pub use ax_types;
pub use ax_types::{
    app_id,
    service::{Order, QueryRequest},
};
pub use futures::future::BoxFuture;
pub use std::{
    future::Future,
    path::PathBuf,
    pin::Pin,
    process::Output,
    sync::{
        mpsc::{self, Receiver},
        Arc, RwLock,
    },
};

pub type EventServiceLock = Arc<RwLock<Option<EventService>>>;

pub struct AxThreadParams {
    pub storage_dir: PathBuf,
    pub bind_to: BindTo,
}

pub fn get_event_service(ax_service_lock: &EventServiceLock) -> anyhow::Result<EventService> {
    let service_read_lock_guard = ax_service_lock
        .read()
        .map_err(|_| anyhow::anyhow!("thread poison error"))?;

    match &*service_read_lock_guard {
        None => Err(anyhow::anyhow!("service is still being initialized")),
        Some(x) => Ok(x.clone()),
    }
}

fn ax_thread_fn(
    rec_path: Receiver<AxThreadParams>,
    lock: Arc<RwLock<Option<EventService>>>,
    is_alive: impl Fn() -> bool + Send + 'static,
) -> anyhow::Result<()> {
    use ax_core::node::{shutdown_ceremony, ApplicationState, Runtime};
    let AxThreadParams { bind_to, storage_dir } = rec_path.recv()?;

    println!("actyx-data running in {}", storage_dir.display());
    std::fs::create_dir_all(storage_dir.clone())
        .with_context(|| format!("creating working directory `{:?}`", storage_dir.display()))?;
    let storage_dir = storage_dir.canonicalize()?;

    #[cfg(any(target_os = "linux", target_os = "macos"))]
    let runtime = Runtime::Linux;
    #[cfg(target_os = "windows")]
    let runtime = Runtime::Windows;

    let actyx = ApplicationState::spawn(storage_dir, runtime, bind_to, false, false)?;

    {
        // scope service_rw_lock so that it closes after this block
        let mut service_rw_lock = lock.write().map_err(|e| {
            eprintln!("unlikely service_rw_lock poison_error {:?}", e);
            anyhow::anyhow!("unlikely service_rw_lock poison_error {:?}", e)
        })?;
        *service_rw_lock = Some(actyx.event_service.clone());
    }

    shutdown_ceremony(actyx)?;

    Ok(())
}

type Req<'a, O> = Box<dyn FnOnce(EventService) -> BoxFuture<'a, O>>;
type Exec<O> = dyn FnOnce(Req<O>) -> O + Send + 'static;

pub struct EventServiceBlockingRef(EventService, tokio::runtime::Runtime);

impl EventServiceBlockingRef {
    pub fn exec<'a, O>(self, req: impl FnOnce(EventService) -> BoxFuture<'a, O>) -> O {
        let EventServiceBlockingRef(service, runtime) = self;
        runtime.block_on(req(service))
    }
}

impl TryFrom<&EventServiceLock> for EventServiceBlockingRef {
    type Error = anyhow::Error;

    fn try_from(value: &EventServiceLock) -> Result<Self, Self::Error> {
        let service = get_event_service(value)?;
        let runtime =
            tokio::runtime::Runtime::new().map_err(|x| anyhow::anyhow!("failed initializing runtime {:?}", x))?;
        Ok(EventServiceBlockingRef(service, runtime))
    }
}

pub fn init(
    is_alive: impl Fn() -> bool + Send + 'static,
) -> (
    EventServiceLock,
    std::thread::JoinHandle<Result<(), anyhow::Error>>,
    impl Fn(AxThreadParams) -> Result<(), mpsc::SendError<AxThreadParams>> + Send + 'static,
) {
    let (param_send, param_receive) = mpsc::channel::<AxThreadParams>();
    let lock: EventServiceLock = Default::default();
    let thread = {
        let ax_service_lock = lock.clone();
        std::thread::spawn(move || ax_thread_fn(param_receive, ax_service_lock.clone(), is_alive))
    };
    let init = move |params: AxThreadParams| param_send.send(params);

    (lock, thread, init)
}
