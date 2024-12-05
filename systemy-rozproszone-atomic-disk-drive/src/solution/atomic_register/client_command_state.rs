use super::utils::SuccessCallback;
use crate::*;
use std::collections::{HashMap, HashSet};
use std::sync::Arc;
use uuid::Uuid;

use log::error;

pub(crate) struct ClientCommandState {
    header: SystemCommandHeader,
    state: ClientCommandEnum,
    callback: SuccessCallback,
    request_identifier: u64,
}

impl ClientCommandState {
    pub(crate) fn new_in_read_proc(
        self_ident: u8,
        read_ident: u64,
        sector_idx: SectorIdx,
        callback: SuccessCallback,
        request_identifier: u64,
        writeval: Option<SectorVec>,
    ) -> Self {
        let state = ClientCommandEnum::ReadProc {
            readlist: HashMap::new(),
            writeval,
        };
        Self {
            header: SystemCommandHeader {
                process_identifier: self_ident,
                msg_ident: Uuid::new_v4(),
                read_ident,
                sector_idx,
            },
            state,
            callback,
            request_identifier,
        }
    }

    pub(crate) fn is_compatible(&self, header: &SystemCommandHeader) -> bool {
        if header.msg_ident == self.header.msg_ident {
            if header.read_ident != self.header.read_ident
                || header.sector_idx != self.header.sector_idx
            {
                error!("Got message with matching uuid but broken other data");
                return false;
            }
            true
        } else {
            false
        }
    }

    pub(crate) fn build_message(&self, content: SystemRegisterCommandContent) -> Broadcast {
        Broadcast {
            cmd: Arc::new(SystemRegisterCommand {
                header: self.header.clone(),
                content,
            }),
        }
    }

    pub(crate) fn build_self_message(&self, content: SystemRegisterCommandContent) -> Send {
        Send {
            target: self.header.process_identifier,
            cmd: Arc::new(SystemRegisterCommand {
                header: self.header.clone(),
                content,
            }),
        }
    }

    pub(crate) fn get(&mut self) -> &mut ClientCommandEnum {
        &mut self.state
    }

    pub(crate) fn put_write_proc(&mut self, readval: Option<SectorVec>) {
        self.state = ClientCommandEnum::WriteProc {
            acklist: HashSet::new(),
            readval,
        };
    }

    pub(crate) async fn finish(self) {
        let mut val = None;
        match self.state {
            ClientCommandEnum::WriteProc { readval, .. } => {
                val = readval;
            }
            _ => {
                error!("Finish called on non finished client command");
            }
        }

        let op_return = if let Some(val) = val {
            OperationReturn::Read(ReadReturn { read_data: val })
        } else {
            OperationReturn::Write
        };
        (self.callback)(OperationSuccess {
            request_identifier: self.request_identifier,
            op_return,
        })
        .await;
    }

    pub(crate) fn get_sector_idx(&self) -> SectorIdx {
        self.header.sector_idx
    }
}

pub(crate) enum ClientCommandEnum {
    ReadProc {
        readlist: HashMap<u8, (u64, u8, SectorVec)>,
        writeval: Option<SectorVec>,
    },
    WriteProc {
        acklist: HashSet<u8>,
        readval: Option<SectorVec>,
    },
}
