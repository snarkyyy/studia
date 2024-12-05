mod client_command_state;
mod metadata;
pub mod utils;

use crate::*;
use client_command_state::{ClientCommandEnum, ClientCommandState};
use log::*;
use metadata::SolutionAtomicRegisterData;
use std::sync::Arc;
use utils::SuccessCallback;

use metadata::SectorMetadata;

pub(crate) struct SolutionAtomicRegister {
    self_ident: u8,
    data: SolutionAtomicRegisterData,
    register_client: Arc<dyn RegisterClient>,
    processes_count: u8,
    cmd_state: Option<ClientCommandState>,
}

#[async_trait::async_trait]
impl AtomicRegister for SolutionAtomicRegister {
    async fn client_command(
        &mut self,
        cmd: ClientRegisterCommand,
        success_callback: SuccessCallback,
    ) {
        assert!(self.cmd_state.is_none());
        let request_identifier = cmd.header.request_identifier;
        let sector_idx = cmd.header.sector_idx;
        let mut writeval = None;
        if let ClientRegisterCommandContent::Write { data } = cmd.content {
            writeval = Some(data);
        }
        let mut rid = self.data.get_rid().await;
        self.data.put_rid(rid + 1).await;
        rid += 1;
        self.cmd_state = Some(ClientCommandState::new_in_read_proc(
            self.self_ident,
            rid,
            sector_idx,
            success_callback,
            request_identifier,
            writeval,
        ));
        let meta = self.data.get_meta(sector_idx).await;
        let val = self.data.get_val(sector_idx).await;
        self.register_client
            .broadcast(
                self.cmd_state
                    .as_ref()
                    .unwrap()
                    .build_message(SystemRegisterCommandContent::ReadProc),
            )
            .await;

        // SolutionRegisterClient doesn't broadcast self messages.
        self.add_answer(
            self.self_ident,
            SystemRegisterCommandContent::Value {
                timestamp: meta.ts,
                write_rank: meta.wr,
                sector_data: val,
            },
        )
        .await;
    }

    async fn system_command(&mut self, cmd: SystemRegisterCommand) {
        if utils::is_proc_command(&cmd) {
            self.give_answer(cmd).await;
        } else if let Some(ref mut cmd_state) = self.cmd_state {
            if cmd_state.is_compatible(&cmd.header) {
                self.add_answer(cmd.header.process_identifier, cmd.content)
                    .await;
            } else {
                trace!("atomic_register: Got answer incompatible with the current client command.");
            }
        } else {
            trace!(
                "atomic_register: Got answer but there is no client command (probably finished)."
            );
        }
    }
}

impl SolutionAtomicRegister {
    async fn give_answer(&mut self, cmd: SystemRegisterCommand) {
        let sector_idx = cmd.header.sector_idx;
        match cmd.content {
            SystemRegisterCommandContent::ReadProc => {
                let meta = self.data.get_meta(sector_idx).await;
                let data = self.data.get_val(sector_idx).await;
                self.register_client
                    .send(build_answer(
                        self.self_ident,
                        &cmd,
                        SystemRegisterCommandContent::Value {
                            timestamp: meta.ts,
                            write_rank: meta.wr,
                            sector_data: data,
                        },
                    ))
                    .await;
            }
            SystemRegisterCommandContent::WriteProc {
                timestamp,
                write_rank,
                ref data_to_write,
            } => {
                let meta = self.data.get_meta(sector_idx).await;
                if (timestamp, write_rank) > (meta.ts, meta.wr) {
                    self.data
                        .put_val_and_meta(
                            sector_idx,
                            data_to_write.clone(),
                            &SectorMetadata {
                                ts: timestamp,
                                wr: write_rank,
                            },
                        )
                        .await;
                }
                self.register_client
                    .send(build_answer(
                        self.self_ident,
                        &cmd,
                        SystemRegisterCommandContent::Ack,
                    ))
                    .await;
            }
            _ => {
                error!("atomic_register: give_answer got incorrect message type");
            }
        }
    }

    async fn add_ack(&mut self, process_id: u8) {
        let state = self.cmd_state.as_mut().unwrap();
        match state.get() {
            ClientCommandEnum::WriteProc { acklist, .. } => {
                acklist.insert(process_id);
                if 2 * acklist.len() > (self.processes_count as usize) {
                    let msg = state.build_self_message(SystemRegisterCommandContent::Ack);
                    self.cmd_state.take().unwrap().finish().await;
                    // Send self message so SolutionRegisterClient will stop resending WriteProc
                    self.register_client.send(msg).await;
                }
            }
            _ => {
                error!("atomic_register: add_ack called with incorrect state");
            }
        }
    }

    pub async fn add_answer(&mut self, process_id: u8, cmd: SystemRegisterCommandContent) {
        let state = self.cmd_state.as_mut().unwrap();
        match (cmd, state.get()) {
            (SystemRegisterCommandContent::Ack, ClientCommandEnum::WriteProc { .. }) => {
                self.add_ack(process_id).await;
            }
            (
                SystemRegisterCommandContent::Value {
                    timestamp,
                    write_rank,
                    sector_data,
                },
                ClientCommandEnum::ReadProc { readlist, writeval },
            ) => {
                readlist.insert(process_id, (timestamp, write_rank, sector_data));
                if 2 * readlist.len() > (self.processes_count as usize) {
                    let mut highest = readlist
                        .values()
                        .max_by_key(|x| (x.0, x.1))
                        .unwrap()
                        .clone();
                    let mut readval = Some(highest.2.clone());
                    if let Some(val) = writeval.take() {
                        readval = None;
                        highest = (highest.0 + 1, self.self_ident, val);
                        self.data
                            .put_val_and_meta(
                                state.get_sector_idx(),
                                highest.2.clone(),
                                &SectorMetadata {
                                    ts: highest.0,
                                    wr: highest.1,
                                },
                            )
                            .await;
                    }
                    state.put_write_proc(readval);
                    self.register_client
                        .broadcast(
                            state.build_message(SystemRegisterCommandContent::WriteProc {
                                timestamp: highest.0,
                                write_rank: highest.1,
                                data_to_write: highest.2,
                            }),
                        )
                        .await;
                    // SolutionRegisterClient doesn't broadcast self messages.
                    self.add_ack(self.self_ident).await;
                }
            }
            (SystemRegisterCommandContent::Value { .. }, ClientCommandEnum::WriteProc { .. }) => {}
            _ => {
                error!("atomic_register: add_answer called with incorrect message");
            }
        }
    }
}

pub async fn build_atomic_register(
    self_ident: u8,
    metadata: Box<dyn StableStorage>,
    register_client: Arc<dyn RegisterClient>,
    sectors_manager: Arc<dyn SectorsManager>,
    processes_count: u8,
) -> Box<dyn AtomicRegister> {
    let data = SolutionAtomicRegisterData::new(metadata, sectors_manager);
    Box::new(SolutionAtomicRegister {
        self_ident,
        data: data,
        register_client,
        processes_count,
        cmd_state: None,
    })
}

fn build_answer(
    self_ident: u8,
    prev: &SystemRegisterCommand,
    content: SystemRegisterCommandContent,
) -> crate::Send {
    crate::Send {
        cmd: Arc::new(SystemRegisterCommand {
            header: SystemCommandHeader {
                process_identifier: self_ident,
                msg_ident: prev.header.msg_ident,
                read_ident: prev.header.read_ident,
                sector_idx: prev.header.sector_idx,
            },
            content: content,
        }),
        target: prev.header.process_identifier,
    }
}
