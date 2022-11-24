use crate::{formats::ExternalEvent, node_settings::Settings, NodeError, ShutdownReason};
use acto::{AcTokio, ActoCell, ActoMsgSuper, ActoRef, ActoRuntime, SupervisionRef};
use crossbeam::channel::Sender;
use std::{any::Any, sync::Arc};

pub enum ActorCommand {
    NewSettings(Settings),
    Supervise(SupervisionRef<ComponentCommand, <AcTokio as ActoRuntime>::ActoHandle<anyhow::Result<()>>>),
}

pub enum ComponentCommand {
    NewSettings(Settings),
}

pub struct Actors {
    tokio: AcTokio,
    supervisor: ActoRef<ActorCommand>,
}

impl Actors {
    pub fn new(node_tx: Sender<ExternalEvent>) -> anyhow::Result<Self> {
        let tokio = AcTokio::new("Node", 2)?;
        let node2 = node_tx.clone();
        let supervisor = tokio.spawn_actor("super", move |cell| supervisor(cell, node_tx)).me;
        node2
            .send(ExternalEvent::RegisterActors(supervisor.clone()))
            .expect("capacity at least 1");
        Ok(Self { tokio, supervisor })
    }

    pub fn rt(&self) -> &AcTokio {
        &self.tokio
    }

    pub fn supervise(
        &self,
        actor: SupervisionRef<ComponentCommand, <AcTokio as ActoRuntime>::ActoHandle<anyhow::Result<()>>>,
    ) {
        self.supervisor.send(ActorCommand::Supervise(actor));
    }
}

async fn supervisor(
    mut cell: ActoCell<ActorCommand, impl ActoRuntime, anyhow::Result<()>>,
    node_tx: Sender<ExternalEvent>,
) {
    let mut supervised = Vec::<ActoRef<ComponentCommand>>::new();
    while let Some(msg) = cell.recv().await.has_senders() {
        match msg {
            ActoMsgSuper::Message(ActorCommand::NewSettings(settings)) => {
                for ar in &supervised {
                    ar.send(ComponentCommand::NewSettings(settings.clone()));
                }
            }
            ActoMsgSuper::Message(ActorCommand::Supervise(ar)) => {
                supervised.push(cell.supervise(ar));
            }
            ActoMsgSuper::Supervision { id, name, result } => {
                let result = result
                    .map_err(fmt_panic)
                    .and_then(|result| result.map_err(|err| format!("{:#}", err)));
                supervised.retain(|ar| ar.id() != id);
                match result {
                    Ok(_) => tracing::error!("actor {} stopped", name),
                    Err(e) => tracing::error!("actor {} died: {}", name, e),
                }
                break;
            }
        }
    }
    node_tx
        .send(ExternalEvent::ShutdownRequested(ShutdownReason::Internal(
            NodeError::InternalError(Arc::new(anyhow::anyhow!("actor failed"))),
        )))
        .ok();
}

fn fmt_panic(err: Box<dyn Any + Send + 'static>) -> String {
    err.downcast::<&'static str>()
        .map(|s| (*s).to_owned())
        .or_else(|err| err.downcast::<String>().map(|s| *s))
        .unwrap_or_else(|_| "unknown panic".to_owned())
}
