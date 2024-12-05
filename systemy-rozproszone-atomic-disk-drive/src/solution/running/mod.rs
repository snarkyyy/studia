mod ar_actor;
mod context;
mod paths_manager;

use crate::*;
use context::Context;
use log::*;

use tokio;
use tokio::sync::mpsc::{self, UnboundedReceiver, UnboundedSender};

use ar_actor::AtomicRegisterActorHandler;
use tokio::net::tcp::{OwnedReadHalf, OwnedWriteHalf};
use tokio::net::TcpListener;

use paths_manager::PathsManager;

use crate::solution::register_client::build_register_client;
use crate::solution::transfer;
use crate::solution::transfer::command_type::ClientCommandType;

use tokio::io::BufReader;

pub(crate) const NUMBER_OF_WORKERS: usize = 16;

pub(crate) async fn run_register_process(config: Configuration) {
    let ctx = Context::new(config);
    let listener = TcpListener::bind(ctx.self_addr())
        .await
        .expect("Couldn't bind");

    let register_client = build_register_client(
        ctx.self_rank().clone(),
        ctx.tcp_locations().clone(),
        ctx.hmac_system_key(),
    )
    .await;

    let mut paths_manager = PathsManager::new(ctx.storage_dir().clone()).await;

    let mut handlers = vec![];
    for i in 0..NUMBER_OF_WORKERS {
        handlers.push(
            AtomicRegisterActorHandler::new(
                &ctx,
                register_client.clone(),
                paths_manager.get_stable_storage(i as u8).await,
                paths_manager.get_sectors_manager().await,
            )
            .await,
        );
    }

    listen(ctx, listener, handlers).await;
}

async fn listen(ctx: Context, listener: TcpListener, handlers: Vec<AtomicRegisterActorHandler>) {
    while let Ok((stream, _)) = listener.accept().await {
        let (read_stream, write_stream) = stream.into_split();
        let (success_rx, success_tx) = mpsc::unbounded_channel();
        let (failure_rx, failure_tx) = mpsc::unbounded_channel();
        tokio::spawn(run_command_reader_actor(
            read_stream,
            handlers.clone(),
            success_rx,
            failure_rx,
            ctx.processes_count(),
            ctx.n_sectors(),
            ctx.hmac_system_key().clone(),
            ctx.hmac_client_key().clone(),
        ));
        tokio::spawn(run_command_writer_actor(
            write_stream,
            success_tx,
            failure_tx,
            ctx.hmac_client_key().clone(),
        ));
    }
}

async fn run_command_reader_actor(
    read_stream: OwnedReadHalf,
    handlers: Vec<AtomicRegisterActorHandler>,
    success_rx: UnboundedSender<OperationSuccess>,
    failure_rx: UnboundedSender<(u64, StatusCode, ClientCommandType)>,
    processes_count: u8,
    n_sectors: u64,
    hmac_system_key: [u8; 64],
    hmac_client_key: [u8; 32],
) {
    let mut reader = BufReader::new(read_stream);
    while let Ok((cmd, valid)) =
        transfer::deserialize_register_command(&mut reader, &hmac_system_key, &hmac_client_key)
            .await
    {
        match (cmd, valid) {
            (RegisterCommand::Client(cmd), false) => {
                if failure_rx
                    .send((
                        cmd.header.request_identifier,
                        StatusCode::AuthFailure,
                        ClientCommandType::new_from_command(&cmd),
                    ))
                    .is_err()
                {
                    trace!("Failed to send auth failure to the sending actor");
                };
            }
            (RegisterCommand::Client(cmd), true) => {
                if !((0..(n_sectors)).contains(&cmd.header.sector_idx)) {
                    if failure_rx
                        .send((
                            cmd.header.request_identifier,
                            StatusCode::InvalidSectorIndex,
                            ClientCommandType::new_from_command(&cmd),
                        ))
                        .is_err()
                    {
                        trace!("Failed to send sector index failure to the sending actor");
                    }
                } else {
                    handlers[(cmd.header.sector_idx % (NUMBER_OF_WORKERS as u64)) as usize]
                        .client(cmd, success_rx.clone())
                        .await;
                }
            }
            (RegisterCommand::System(cmd), true) => {
                if !((1..=processes_count).contains(&cmd.header.process_identifier)) {
                    error!("Invalid process_identifier");
                } else if !((0..(n_sectors)).contains(&cmd.header.sector_idx)) {
                    error!("Invalid sector_idx");
                } else {
                    handlers[(cmd.header.sector_idx % (NUMBER_OF_WORKERS as u64)) as usize]
                        .system(cmd)
                        .await;
                }
            }
            _ => {
                trace!("Ignored command.");
            }
        }
    }
}

async fn run_command_writer_actor(
    mut write_stream: OwnedWriteHalf,
    mut success_tx: UnboundedReceiver<OperationSuccess>,
    mut failure_tx: UnboundedReceiver<(u64, StatusCode, ClientCommandType)>,
    hmac_client_key: [u8; 32],
) {
    loop {
        tokio::select! {
            Some(result) = success_tx.recv() => {
                if transfer::deserialize_response_success(
                    &mut write_stream,
                    &hmac_client_key,
                    result.request_identifier,
                    result.op_return,
                ).await.is_err() {
                    break;
                }
            },
            Some((request_identifier, code, cct)) = failure_tx.recv() => {
                if transfer::deserialize_response_failure(
                    &mut write_stream,
                    &hmac_client_key,
                    request_identifier,
                    code,
                    cct,
                ).await.is_err() {
                    break;
                }
            },
            else => {
                break;
            }
        }
    }
}
