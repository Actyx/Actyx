use crate::m;
use actyxos_sdk::{service::EventService, AppManifest, HttpClient, NodeId, Url};
use anyhow::{anyhow, Result};
use async_std::task::block_on;
use netsim_embed::{Machine, Namespace};
use netsim_embed::{MachineId, Netsim};
use std::borrow::Borrow;
use std::collections::BTreeMap;
use std::future::Future;
use std::str::FromStr;
use swarm_cli::{Command, Event};
use util::pinned_resource::PinnedResource;

pub struct Api {
    machines: BTreeMap<MachineId, ApiClient>,
}

impl Api {
    pub fn new<E>(sim: &mut Netsim<Command, E>, app_manifest: AppManifest) -> Result<Self>
    where
        E: Borrow<Event> + FromStr<Err = anyhow::Error> + Send + 'static,
    {
        let machines = sim
            .machines_mut()
            .iter_mut()
            .map(move |machine| {
                let id = machine.id();
                let client = ApiClient::from_machine(machine, app_manifest.clone())?;
                Ok((id, client))
            })
            .collect::<Result<_>>()?;
        Ok(Self { machines })
    }

    pub async fn run<F, T, Fut>(&self, machine: MachineId, f: F) -> Result<T>
    where
        F: FnOnce(ApiClient) -> Fut,
        Fut: Future<Output = Result<T>> + Send,
    {
        f(self.machines[&machine].clone()).await
    }
}

#[derive(Clone)]
pub struct ApiClient(PinnedResource<HttpClient>);
impl ApiClient {
    pub fn new(origin: Url, app_manifest: AppManifest, namespace: Namespace) -> Self {
        Self(PinnedResource::new(move || {
            if let Err(e) = namespace.enter() {
                tracing::error!("cannot enter namespace {}: {}", namespace, e);
                panic!();
            }
            block_on(HttpClient::new(origin, app_manifest)).expect("cannot create")
        }))
    }
    pub async fn node_id(&self) -> Result<NodeId> {
        self.0.spawn_mut(|c| block_on(c.node_id())).await.unwrap()
    }
    pub fn from_machine<E: Borrow<Event> + FromStr<Err = anyhow::Error> + Send + 'static>(
        machine: &mut Machine<Command, E>,
        app_manifest: AppManifest,
    ) -> Result<Self> {
        machine.send(Command::ApiPort);
        let api_port = block_on(machine.select(|ev| m!(ev.borrow(), Event::ApiPort(port) => *port)))
            .ok_or_else(|| anyhow!("machine died"))?
            .ok_or_else(|| anyhow!("api endpoint not configured"))?;

        let origin = Url::parse(&*format!("http://{}:{}", machine.addr(), api_port))?;
        let namespace = machine.namespace();
        Ok(ApiClient::new(origin, app_manifest, namespace))
    }
}

#[async_trait::async_trait]
impl EventService for ApiClient {
    async fn offsets(&self) -> Result<actyxos_sdk::service::OffsetsResponse> {
        self.0.spawn_mut(|c| block_on(c.offsets())).await.unwrap()
    }

    async fn publish(
        &self,
        request: actyxos_sdk::service::PublishRequest,
    ) -> Result<actyxos_sdk::service::PublishResponse> {
        self.0.spawn_mut(|c| block_on(c.publish(request))).await.unwrap()
    }

    async fn query(
        &self,
        request: actyxos_sdk::service::QueryRequest,
    ) -> Result<futures::stream::BoxStream<'static, actyxos_sdk::service::QueryResponse>> {
        self.0.spawn_mut(|c| block_on(c.query(request))).await.unwrap()
    }

    async fn subscribe(
        &self,
        request: actyxos_sdk::service::SubscribeRequest,
    ) -> Result<futures::stream::BoxStream<'static, actyxos_sdk::service::SubscribeResponse>> {
        self.0.spawn_mut(|c| block_on(c.subscribe(request))).await.unwrap()
    }

    async fn subscribe_monotonic(
        &self,
        request: actyxos_sdk::service::SubscribeMonotonicRequest,
    ) -> Result<futures::stream::BoxStream<'static, actyxos_sdk::service::SubscribeMonotonicResponse>> {
        self.0
            .spawn_mut(|c| block_on(c.subscribe_monotonic(request)))
            .await
            .unwrap()
    }
}
