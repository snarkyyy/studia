use crate::*;
use crate::{SystemRegisterCommand, SystemRegisterCommandContent};
use std::future::Future;
use std::pin::Pin;

pub(crate) fn is_proc_command(cmd: &SystemRegisterCommand) -> bool {
    match cmd.content {
        SystemRegisterCommandContent::ReadProc { .. }
        | SystemRegisterCommandContent::WriteProc { .. } => true,
        _ => false,
    }
}

pub(crate) fn is_answer_command(cmd: &SystemRegisterCommand) -> bool {
    return !is_proc_command(cmd);
}

pub(crate) type SuccessCallback = Box<
    dyn FnOnce(OperationSuccess) -> Pin<Box<dyn Future<Output = ()> + std::marker::Send>>
        + std::marker::Send
        + Sync,
>;
