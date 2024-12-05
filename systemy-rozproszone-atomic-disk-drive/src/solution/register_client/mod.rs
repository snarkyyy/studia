mod connector;
mod connectors_manager;
mod resender;

use crate::solution::atomic_register::utils as arutils;
use crate::*;
use std::sync::Arc;

use self::connectors_manager::ConnectorsManager;
use self::resender::ResenderActorHandle;

/// Unlike the trait specification this client won't send
/// self messages, furthermore SolutionRegisterClient is
/// implemented such that it doesn't need such messages.
/// Stubborness is done by resending only the newest
/// message for every UUID. Only question messages are resend.
pub(crate) struct SolutionRegisterClient {
    resender: ResenderActorHandle,
    manager: ConnectorsManager,
}

#[async_trait::async_trait]
impl RegisterClient for SolutionRegisterClient {
    async fn send(&self, msg: Send) {
        assert!(arutils::is_answer_command(&msg.cmd));
        assert!(msg.target >= 1);
        self.manager.send(msg.clone());
        self.resender.process_send(msg);
    }

    async fn broadcast(&self, msg: Broadcast) {
        assert!(arutils::is_proc_command(&msg.cmd));
        self.manager.broadcast(msg.clone());
        self.resender.process_broadcast(msg);
    }
}

pub(crate) async fn build_register_client(
    self_rank: u8,
    tcp_locations: Vec<(String, u16)>,
    hmac_system_key: &[u8; 64],
) -> Arc<dyn RegisterClient> {
    let manager = ConnectorsManager::new(self_rank, &tcp_locations, hmac_system_key);
    let resender = ResenderActorHandle::new(manager.clone());
    Arc::new(SolutionRegisterClient { resender, manager })
}
