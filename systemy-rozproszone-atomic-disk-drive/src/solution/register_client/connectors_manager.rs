use super::connector::ConnectorActorHandle;
use crate::*;
use std::ops::Deref;

#[derive(Clone)]
pub(crate) struct ConnectorsManager {
    handles: Vec<Option<ConnectorActorHandle>>,
}

impl ConnectorsManager {
    pub(crate) fn new(
        self_ident: u8,
        tcp_locations: &Vec<(String, u16)>,
        hmac_key: &[u8; 64],
    ) -> Self {
        let mut handles = vec![];
        for (i, loc) in tcp_locations.iter().enumerate() {
            if i + 1 == (self_ident as usize) {
                handles.push(None);
            } else {
                handles.push(Some(ConnectorActorHandle::new(hmac_key, loc)));
            }
        }
        ConnectorsManager { handles }
    }

    pub(crate) fn send(&self, cmd: Send) {
        if let Some(handle) = &self.handles[(cmd.target - 1) as usize] {
            handle.send(cmd.cmd.deref().to_owned());
        }
    }

    pub(crate) fn broadcast(&self, cmd: Broadcast) {
        for opt in self.handles.iter() {
            if let Some(handle) = opt {
                handle.send(cmd.cmd.deref().to_owned());
            }
        }
    }
}
