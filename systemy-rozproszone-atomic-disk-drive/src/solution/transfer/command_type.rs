use crate::{
    domain::{ClientRegisterCommandContent, RegisterCommand, SystemRegisterCommandContent},
    ClientRegisterCommand, OperationReturn,
};

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) enum CommandType {
    Client(ClientCommandType),
    System(SystemCommandType),
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) enum ClientCommandType {
    Read = 0x01,
    Write = 0x02,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) enum SystemCommandType {
    ReadProc = 0x03,
    Value = 0x04,
    WriteProc = 0x05,
    Ack = 0x06,
}

impl ClientCommandType {
    pub(crate) fn try_new(value: u8) -> Option<ClientCommandType> {
        type CCT = ClientCommandType;
        match value {
            x if x == (CCT::Read as u8) => Some(CCT::Read),
            x if x == (CCT::Write as u8) => Some(CCT::Write),
            _ => None,
        }
    }

    pub(crate) fn new_from_command(command: &ClientRegisterCommand) -> Self {
        match command.content {
            ClientRegisterCommandContent::Read => ClientCommandType::Read,
            ClientRegisterCommandContent::Write { .. } => ClientCommandType::Write,
        }
    }
}

impl SystemCommandType {
    pub(crate) fn try_new(value: u8) -> Option<SystemCommandType> {
        type SCT = SystemCommandType;
        match value {
            x if x == (SCT::ReadProc as u8) => Some(SCT::ReadProc),
            x if x == (SCT::Value as u8) => Some(SCT::Value),
            x if x == (SCT::WriteProc as u8) => Some(SCT::WriteProc),
            x if x == (SCT::Ack as u8) => Some(SCT::Ack),
            _ => None,
        }
    }
}

impl CommandType {
    pub(crate) fn try_new(value: u8) -> Option<CommandType> {
        if let Some(cct) = ClientCommandType::try_new(value) {
            Some(CommandType::Client(cct))
        } else if let Some(sct) = SystemCommandType::try_new(value) {
            Some(CommandType::System(sct))
        } else {
            None
        }
    }

    pub(crate) fn new_from_command(command: &RegisterCommand) -> CommandType {
        type CRCC = ClientRegisterCommandContent;
        type SRCC = SystemRegisterCommandContent;
        match command {
            RegisterCommand::Client(crc) => match crc.content {
                CRCC::Read => CommandType::Client(ClientCommandType::Read),
                CRCC::Write { .. } => CommandType::Client(ClientCommandType::Write),
            },
            RegisterCommand::System(src) => match src.content {
                SRCC::ReadProc => CommandType::System(SystemCommandType::ReadProc),
                SRCC::Value { .. } => CommandType::System(SystemCommandType::Value),
                SRCC::WriteProc { .. } => CommandType::System(SystemCommandType::WriteProc),
                SRCC::Ack => CommandType::System(SystemCommandType::Ack),
            },
        }
    }

    pub(crate) fn new_from_operation_return(opret: &OperationReturn) -> ClientCommandType {
        match opret {
            OperationReturn::Read(..) => ClientCommandType::Read,
            OperationReturn::Write => ClientCommandType::Write,
        }
    }

    pub(crate) fn value(&self) -> u8 {
        match self {
            CommandType::Client(val) => val.clone() as u8,
            CommandType::System(val) => val.clone() as u8,
        }
    }
}

#[test]
fn test_command_type_constructor() {
    assert_eq!(None, CommandType::try_new(0x00));
    assert_eq!(
        Some(CommandType::Client(ClientCommandType::Read)),
        CommandType::try_new(0x01)
    );
    assert_eq!(
        Some(CommandType::Client(ClientCommandType::Write)),
        CommandType::try_new(0x02)
    );
    assert_eq!(
        Some(CommandType::System(SystemCommandType::ReadProc)),
        CommandType::try_new(0x03)
    );
    assert_eq!(
        Some(CommandType::System(SystemCommandType::Value)),
        CommandType::try_new(0x04)
    );
    assert_eq!(
        Some(CommandType::System(SystemCommandType::WriteProc)),
        CommandType::try_new(0x05)
    );
    assert_eq!(
        Some(CommandType::System(SystemCommandType::Ack)),
        CommandType::try_new(0x06)
    );
    assert_eq!(None, CommandType::try_new(0x07));

    assert_eq!(None, CommandType::try_new(0x41));
}
