pub(crate) mod command_type;
pub(crate) mod message_header;
pub(crate) mod utils;

use crate::domain::*;
use command_type::{ClientCommandType, CommandType, SystemCommandType};
use hmac::{Hmac, Mac};
use message_header::MessageHeader;
use sha2::Sha256;
use std::io::Error;
use tokio::io::{AsyncRead, AsyncReadExt, AsyncWrite, AsyncWriteExt};
use uuid::Uuid;

type HmacSha256 = Hmac<Sha256>;

pub(crate) const SECTOR_LEN: usize = 4096;

pub(crate) async fn deserialize_register_command(
    data: &mut (dyn AsyncRead + std::marker::Send + Unpin),
    hmac_system_key: &[u8; 64],
    hmac_client_key: &[u8; 32],
) -> Result<(RegisterCommand, bool), Error> {
    let header = utils::read_message_prefix_until_valid_command_type(data).await?;
    let command = read_command(data, &header).await?;
    let mut byte_view = vec![];
    let writer: &mut (dyn AsyncWrite + std::marker::Send + Unpin) = &mut byte_view;
    writer.write_all(&MAGIC_NUMBER).await?;
    writer.write_all(&header.to_be_bytes().await).await?;
    write_command(writer, &command).await?;
    let mut mac = HmacSha256::new_from_slice(match header.command_type().unwrap() {
        CommandType::Client(..) => hmac_client_key,
        CommandType::System(..) => hmac_system_key,
    })
    .unwrap();
    mac.update(&byte_view);
    let mut buf = [0; 32];
    data.read_exact(&mut buf).await?;
    return Ok((command, mac.verify_slice(&buf).is_ok()));
}

pub(crate) async fn serialize_register_command(
    cmd: &RegisterCommand,
    writer: &mut (dyn AsyncWrite + std::marker::Send + Unpin),
    hmac_key: &[u8],
) -> Result<(), Error> {
    let mut byte_view = vec![];
    {
        let writer: &mut (dyn AsyncWrite + std::marker::Send + Unpin) = &mut byte_view;
        writer.write_all(&MAGIC_NUMBER).await?;
        let header = MessageHeader::new_from_register_command(cmd);
        writer.write_all(&header.to_be_bytes().await).await?;
        write_command(writer, cmd).await?;
    }
    let mut mac = HmacSha256::new_from_slice(hmac_key).unwrap();
    mac.update(&byte_view);
    writer.write_all(&byte_view).await?;
    writer.write_all(&mac.finalize().into_bytes()).await?;
    Ok(())
}

pub(crate) async fn deserialize_response_failure(
    writer: &mut (dyn AsyncWrite + std::marker::Send + Unpin),
    hmac_key: &[u8; 32],
    request_number: u64,
    status_code: StatusCode,
    client_command_type: ClientCommandType,
) -> Result<(), Error> {
    assert!(status_code != StatusCode::Ok);
    let header = MessageHeader::new_from_client_failure(&status_code, &client_command_type);
    let mut mac = HmacSha256::new_from_slice(hmac_key).unwrap();
    let mut byte_view = vec![];
    {
        let writer: &mut (dyn AsyncWrite + std::marker::Send + Unpin) = &mut byte_view;
        writer.write_all(&MAGIC_NUMBER).await?;
        writer.write(&header.to_be_bytes().await).await?;
        writer.write_u64(request_number).await?;
    }
    mac.update(&byte_view);
    let result = mac.finalize().into_bytes();
    writer.write_all(&byte_view).await?;
    writer.write_all(&result).await?;
    Ok(())
}

pub(crate) async fn deserialize_response_success(
    writer: &mut (dyn AsyncWrite + std::marker::Send + Unpin),
    hmac_key: &[u8; 32],
    request_number: u64,
    opret: OperationReturn,
) -> Result<(), Error> {
    let header =
        MessageHeader::new_from_client_success(&CommandType::new_from_operation_return(&opret));
    let mut mac = HmacSha256::new_from_slice(hmac_key).unwrap();
    let mut byte_view = vec![];
    {
        let writer: &mut (dyn AsyncWrite + std::marker::Send + Unpin) = &mut byte_view;
        writer.write_all(&MAGIC_NUMBER).await?;
        writer.write(&header.to_be_bytes().await).await?;
        writer.write_u64(request_number).await?;
        if let OperationReturn::Read(ReadReturn {
            read_data: SectorVec(data),
        }) = &opret
        {
            writer.write_all(data).await?;
        }
    }
    mac.update(&byte_view);
    let result = mac.finalize().into_bytes();
    writer.write(&byte_view).await?;
    writer.write_all(&result).await?;
    Ok(())
}

async fn read_command(
    data: &mut (dyn AsyncRead + std::marker::Send + Unpin),
    header: &MessageHeader,
) -> Result<RegisterCommand, Error> {
    type CRCC = ClientRegisterCommandContent;
    type SRCC = SystemRegisterCommandContent;
    match header.command_type().unwrap() {
        CommandType::Client(cct) => Ok(RegisterCommand::Client(ClientRegisterCommand {
            header: ClientCommandHeader {
                request_identifier: data.read_u64().await?,
                sector_idx: data.read_u64().await?,
            },
            content: match cct {
                ClientCommandType::Read => CRCC::Read,
                ClientCommandType::Write => CRCC::Write {
                    data: read_sector_vec(data).await?,
                },
            },
        })),
        CommandType::System(sct) => Ok(RegisterCommand::System(SystemRegisterCommand {
            header: SystemCommandHeader {
                process_identifier: header.auxiliary,
                msg_ident: read_uuid(data).await?,
                read_ident: data.read_u64().await?,
                sector_idx: data.read_u64().await?,
            },
            content: match sct {
                SystemCommandType::ReadProc => SRCC::ReadProc,
                SystemCommandType::Value => SRCC::Value {
                    timestamp: data.read_u64().await?,
                    write_rank: (data.read_u64().await? as u8),
                    sector_data: read_sector_vec(data).await?,
                },
                SystemCommandType::WriteProc => SRCC::WriteProc {
                    timestamp: data.read_u64().await?,
                    write_rank: (data.read_u64().await? as u8),
                    data_to_write: read_sector_vec(data).await?,
                },
                SystemCommandType::Ack => SRCC::Ack,
            },
        })),
    }
}

async fn read_sector_vec(
    data: &mut (dyn AsyncRead + std::marker::Send + Unpin),
) -> Result<SectorVec, Error> {
    let mut sector_data: Vec<u8> = vec![0; SECTOR_LEN];
    data.read_exact(&mut sector_data).await?;
    Ok(SectorVec(sector_data))
}

async fn read_uuid(data: &mut (dyn AsyncRead + std::marker::Send + Unpin)) -> Result<Uuid, Error> {
    let mut buf = [0; 16];
    data.read_exact(&mut buf).await?;
    Ok(Uuid::from_bytes(buf))
}

async fn write_command(
    writer: &mut (dyn AsyncWrite + std::marker::Send + Unpin),
    cmd: &RegisterCommand,
) -> Result<(), Error> {
    type SRCC = SystemRegisterCommandContent;
    type CRCC = ClientRegisterCommandContent;
    match cmd {
        RegisterCommand::Client(ClientRegisterCommand { header, content }) => {
            writer.write_u64(header.request_identifier).await?;
            writer.write_u64(header.sector_idx).await?;
            match content {
                CRCC::Read => {}
                CRCC::Write { data } => {
                    write_sector_vec(writer, data).await?;
                }
            }
        }
        RegisterCommand::System(SystemRegisterCommand { header, content }) => {
            write_uuid(writer, &header.msg_ident).await?;
            writer.write_u64(header.read_ident).await?;
            writer.write_u64(header.sector_idx).await?;
            match content {
                SRCC::ReadProc => {}
                SRCC::Value {
                    timestamp,
                    write_rank,
                    sector_data,
                } => {
                    writer.write_u64(*timestamp).await?;
                    writer.write_u64((*write_rank) as u64).await?;
                    write_sector_vec(writer, sector_data).await?;
                }
                SRCC::WriteProc {
                    timestamp,
                    write_rank,
                    data_to_write,
                } => {
                    writer.write_u64(*timestamp).await?;
                    writer.write_u64((*write_rank) as u64).await?;
                    write_sector_vec(writer, data_to_write).await?;
                }
                SRCC::Ack => {}
            }
        }
    }
    Ok(())
}

async fn write_sector_vec(
    writer: &mut (dyn AsyncWrite + std::marker::Send + Unpin),
    sector_vec: &SectorVec,
) -> Result<(), Error> {
    let SectorVec(ref sector_data) = sector_vec;
    writer.write_all(sector_data).await?;
    Ok(())
}

async fn write_uuid(
    writer: &mut (dyn AsyncWrite + std::marker::Send + Unpin),
    uuid: &Uuid,
) -> Result<(), Error> {
    writer.write_all(uuid.as_bytes()).await?;
    Ok(())
}
