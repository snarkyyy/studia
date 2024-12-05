//! Utilities for initial message de/serialization up until the message type field.
use super::command_type::{ClientCommandType, CommandType};
use crate::domain::{RegisterCommand, SystemRegisterCommand};
use crate::*;
use std::io::Cursor;
use tokio::io::{AsyncReadExt, AsyncWriteExt};

const RESPONSE_BIT: u8 = 0x40;

pub(crate) struct MessageHeader {
    padding: u16,
    pub(crate) auxiliary: u8,
    pub(crate) message_type: u8,
}

impl MessageHeader {
    pub(crate) async fn new_from_be_bytes(bytes: &[u8; 4]) -> Self {
        let mut reader = &bytes[..];
        MessageHeader {
            padding: reader.read_u16().await.unwrap(),
            auxiliary: reader.read_u8().await.unwrap(),
            message_type: reader.read_u8().await.unwrap(),
        }
    }

    pub(crate) fn new_from_register_command(command: &RegisterCommand) -> Self {
        MessageHeader {
            padding: 0,
            auxiliary: get_auxiliary(command),
            message_type: CommandType::new_from_command(command).value(),
        }
    }

    pub(crate) fn new_from_client_failure(
        status_code: &StatusCode,
        client_command_type: &ClientCommandType,
    ) -> Self {
        MessageHeader {
            padding: 0,
            auxiliary: (status_code.clone() as u8),
            message_type: (client_command_type.clone() as u8) + RESPONSE_BIT,
        }
    }

    pub(crate) fn new_from_client_success(client_command_type: &ClientCommandType) -> Self {
        MessageHeader {
            padding: 0,
            auxiliary: (StatusCode::Ok as u8),
            message_type: (client_command_type.clone() as u8) + RESPONSE_BIT,
        }
    }

    pub(crate) async fn to_be_bytes(&self) -> [u8; 4] {
        let mut bytes = [0u8; 4];
        let mut writer = Cursor::new(&mut bytes[..]);
        writer.write_u16(self.padding).await.unwrap();
        writer.write_u8(self.auxiliary).await.unwrap();
        writer.write_u8(self.message_type).await.unwrap();
        bytes
    }

    pub(crate) fn command_type(&self) -> Option<CommandType> {
        CommandType::try_new(self.message_type)
    }
}

fn get_auxiliary(command: &RegisterCommand) -> u8 {
    // XXX: might be buggy.
    match command {
        RegisterCommand::System(SystemRegisterCommand { header, .. }) => header.process_identifier,
        _ => 0,
    }
}
