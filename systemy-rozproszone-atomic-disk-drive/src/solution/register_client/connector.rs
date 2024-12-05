use crate::solution::transfer;
use crate::*;
use log::*;
use tokio;
use tokio::net::TcpStream;
use tokio::sync::mpsc::{self, UnboundedReceiver, UnboundedSender};
use tokio::time::{self, Duration};

async fn run_connector_actor(
    hmac_key: [u8; 64],
    location: (String, u16),
    mut rx: UnboundedReceiver<SystemRegisterCommand>,
) {
    loop {
        time::sleep(Duration::from_millis(200)).await;
        if let Ok(mut tcp_stream) = TcpStream::connect(&location).await {
            while let Some(cmd) = rx.recv().await {
                let command = RegisterCommand::System(cmd);

                if transfer::serialize_register_command(&command, &mut tcp_stream, &hmac_key)
                    .await
                    .is_err()
                {
                    break;
                }
            }
        }
        trace!("One of connections broke, reconnecting...");
    }
}

#[derive(Clone)]
pub(crate) struct ConnectorActorHandle {
    tx: UnboundedSender<SystemRegisterCommand>,
}

impl ConnectorActorHandle {
    pub(crate) fn new(hmac_key: &[u8; 64], location: &(String, u16)) -> Self {
        let (tx, rx) = mpsc::unbounded_channel();
        tokio::spawn(run_connector_actor(hmac_key.clone(), location.clone(), rx));
        Self { tx }
    }

    pub(crate) fn send(&self, cmd: SystemRegisterCommand) {
        if self.tx.send(cmd).is_err() {
            error!("One of the connectors stoped working");
        }
    }
}
