use super::context::Context;
use crate::solution::atomic_register::build_atomic_register;
use crate::*;
use log::*;
use std::sync::Arc;
use tokio;
use tokio::sync::mpsc::{self, Receiver, Sender, UnboundedSender};

use crate::solution::running::NUMBER_OF_WORKERS;

use crate::solution::atomic_register::utils::SuccessCallback;

async fn run_atomic_register_actor(
    mut ar: Box<dyn AtomicRegister>,
    mut system_rx: Receiver<SystemRegisterCommand>,
    mut client_rx: Receiver<(ClientRegisterCommand, UnboundedSender<OperationSuccess>)>,
) {
    tokio::spawn(async move {
        let mut accept_client = true;
        let (finish_tx, mut finish_rx) = mpsc::unbounded_channel();
        loop {
            tokio::select! {
                biased;
                Some(()) = finish_rx.recv() => {
                    accept_client = true;
                }
                Some((cmd, result_sender)) = client_rx.recv(), if accept_client  => {
                    accept_client = false;
                    let tx = finish_tx.clone();
                    let callback: SuccessCallback = Box::new(move |op_complete| {
                        Box::pin(async move {
                            if tx.send(()).is_err() {
                                error!("Couldn't inform via finish_tx");
                            }
                            if result_sender.send(op_complete).is_err() {
                                error!("Couldn't send error");
                            }
                        })
                    });
                    ar.client_command(cmd, callback).await;
                }
                Some(msg) = system_rx.recv() => {
                    ar.system_command(msg).await;
                }
                else => {
                    break;
                }
            }
        }
        error!("Atomic register actor is ending");
    });
}

#[derive(Clone)]
pub(crate) struct AtomicRegisterActorHandler {
    system_tx: Sender<SystemRegisterCommand>,
    client_tx: Sender<(ClientRegisterCommand, UnboundedSender<OperationSuccess>)>,
}

impl AtomicRegisterActorHandler {
    pub(crate) async fn new(
        ctx: &Context,
        register_client: Arc<dyn RegisterClient>,
        stable_storage: Box<dyn StableStorage>,
        sectors_manager: Arc<dyn SectorsManager>,
    ) -> Self {
        let (system_tx, system_rx) = mpsc::channel(NUMBER_OF_WORKERS);
        let (client_tx, client_rx) = mpsc::channel(NUMBER_OF_WORKERS);
        let ar = build_atomic_register(
            ctx.self_rank().clone(),
            stable_storage,
            register_client,
            sectors_manager,
            ctx.processes_count(),
        )
        .await;
        tokio::spawn(run_atomic_register_actor(ar, system_rx, client_rx));
        Self {
            system_tx,
            client_tx,
        }
    }

    pub(crate) async fn system(&self, cmd: SystemRegisterCommand) {
        if self.system_tx.send(cmd).await.is_err() {
            error!("Couldn't send system message to the register actor");
        }
    }

    pub(crate) async fn client(
        &self,
        cmd: ClientRegisterCommand,
        result_sender: UnboundedSender<OperationSuccess>,
    ) {
        if self.client_tx.send((cmd, result_sender)).await.is_err() {
            error!("Couldn't send client message to the register actor");
        }
    }
}
