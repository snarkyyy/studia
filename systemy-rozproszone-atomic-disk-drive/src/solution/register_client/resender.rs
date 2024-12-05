use super::connectors_manager::ConnectorsManager;
use crate::*;
use log::*;
use std::collections::HashMap;
use tokio;
use tokio::sync::mpsc::{self, UnboundedReceiver, UnboundedSender};
use tokio::time::{self, Duration};
use uuid::Uuid;

struct ResenderActor {
    resends: HashMap<Uuid, Broadcast>,
    manager: ConnectorsManager,
}

enum ResenderActorMessage {
    ToCancel(Uuid),
    ToSend(Broadcast),
}

impl ResenderActor {
    fn new(manager: ConnectorsManager) -> Self {
        Self {
            resends: HashMap::new(),
            manager,
        }
    }

    fn handle_message(&mut self, msg: ResenderActorMessage) {
        match msg {
            ResenderActorMessage::ToCancel(uuid) => {
                let _ = self.resends.remove(&uuid);
            }
            ResenderActorMessage::ToSend(broadcast) => {
                let uuid = broadcast.cmd.header.msg_ident;
                self.resends.insert(uuid, broadcast);
            }
        }
    }

    fn resend(&mut self) {
        for cmd in self.resends.values() {
            self.manager.broadcast(cmd.clone());
        }
    }
}

async fn run_resender_actor(
    mut actor: ResenderActor,
    mut rx: UnboundedReceiver<ResenderActorMessage>,
) {
    loop {
        tokio::select! {
            _ = time::sleep(Duration::from_millis(500)) => {
                actor.resend();
            }
            Some(msg) = rx.recv() => {
                actor.handle_message(msg);
            }
            else => {
                break;
            }
        }
    }
    error!("Closing resender actor");
}

#[derive(Clone)]
pub(crate) struct ResenderActorHandle {
    tx: UnboundedSender<ResenderActorMessage>,
}

impl ResenderActorHandle {
    pub(crate) fn new(manager: ConnectorsManager) -> Self {
        let (tx, rx) = mpsc::unbounded_channel();
        let actor = ResenderActor::new(manager);
        tokio::spawn(run_resender_actor(actor, rx));
        Self { tx }
    }

    pub(crate) fn process_send(&self, cmd: Send) {
        let msg = ResenderActorMessage::ToCancel(cmd.cmd.header.msg_ident);
        if self.tx.send(msg).is_err() {
            error!("Cannot pass message to resender actor");
        }
    }

    pub(crate) fn process_broadcast(&self, cmd: Broadcast) {
        let msg = ResenderActorMessage::ToSend(cmd);
        if self.tx.send(msg).is_err() {
            error!("Cannot pass message to resender actor");
        }
    }
}
