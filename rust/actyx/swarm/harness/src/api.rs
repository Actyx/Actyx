use crate::m;
use actyx_sdk::{AppManifest, Ax, AxOpts, NodeId, Url};
use anyhow::{anyhow, Result};
use async_std::task::block_on;
use futures::channel::oneshot::Canceled;
use netsim_embed::{Machine, Namespace};
use netsim_embed::{MachineId, Netsim};
use std::borrow::Borrow;
use std::collections::BTreeMap;
use std::fmt::Display;
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
        E: Borrow<Event> + FromStr<Err = anyhow::Error> + Display + Send + 'static,
    {
        let machines = sim
            .machines_mut()
            .iter_mut()
            .map(move |machine| {
                let id = machine.id();
                let client = ApiClient::from_machine(machine, app_manifest.clone(), None)?;
                Ok((id, client))
            })
            .collect::<Result<_>>()?;
        Ok(Self { machines })
    }

    pub fn with_port<E>(sim: &mut Netsim<Command, E>, app_manifest: AppManifest, port: u16) -> Result<Self>
    where
        E: Borrow<Event> + FromStr<Err = anyhow::Error> + Display + Send + 'static,
    {
        let machines = sim
            .machines_mut()
            .iter_mut()
            .map(move |machine| {
                let id = machine.id();
                let client = ApiClient::from_machine(machine, app_manifest.clone(), Some(port))?;
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
pub struct ApiClient(PinnedResource<Ax>);
impl ApiClient {
    pub fn new(url: Url, manifest: AppManifest, namespace: Namespace) -> Self {
        Self(PinnedResource::new(move || {
            if let Err(e) = namespace.enter() {
                tracing::error!("cannot enter namespace {}: {}", namespace, e);
                panic!();
            }
            tracing::info!(
                "api {} in namespace {} ({})",
                url,
                Namespace::current().unwrap(),
                namespace
            );
            block_on(Ax::new(AxOpts { url, manifest })).expect("cannot create")
        }))
    }
    pub async fn node_id(&self) -> NodeId {
        self.0.spawn_mut(|c| c.node_id()).await.unwrap()
    }
    pub fn from_machine<E: Borrow<Event> + FromStr<Err = anyhow::Error> + Send + 'static>(
        machine: &mut Machine<Command, E>,
        app_manifest: AppManifest,
        port: Option<u16>,
    ) -> Result<Self> {
        let api_port = match port {
            Some(p) => p,
            None => {
                machine.send(Command::ApiPort);
                block_on(machine.select(|ev| m!(ev.borrow(), Event::ApiPort(port) => *port)))
                    .ok_or_else(|| anyhow!("machine died"))?
                    .ok_or_else(|| anyhow!("api endpoint not configured"))?
            }
        };

        let origin = Url::parse(&format!("http://{}:{}", machine.addr(), api_port))?;
        let namespace = machine.namespace();
        Ok(ApiClient::new(origin, app_manifest, namespace))
    }

    pub async fn offsets(&self) -> Result<actyx_sdk::service::OffsetsResponse> {
        self.0.spawn_mut(|c| block_on(c.offsets())).await.unwrap()
    }

    pub fn execute<U, F>(&self, f: F) -> impl Future<Output = Result<U, Canceled>>
    where
        U: Send + 'static,
        F: FnOnce(&mut Ax) -> U + Send + 'static,
    {
        self.0.spawn_mut(f)
    }
}
