use super::message_header::MessageHeader;
use crate::domain::MAGIC_NUMBER;
use log::{trace, warn};
use std::io::Error;
use tokio::io::{AsyncRead, AsyncReadExt};

async fn read_until_magic_number(data: &mut (dyn AsyncRead + Send + Unpin)) -> Result<(), Error> {
    let mut buf = [0u8; 4];
    let mut traced = false;
    let mut count = 0;
    while buf[0..4] != MAGIC_NUMBER {
        if !traced && count >= 4 {
            traced = true;
            trace!("Garbage bytes found while reading");
        }
        buf[0..4].rotate_left(1);
        buf[3] = data.read_u8().await?;
        count += 1;
    }
    Ok(())
}

pub(crate) async fn read_message_prefix_until_valid_command_type(
    data: &mut (dyn AsyncRead + Send + Unpin),
) -> Result<MessageHeader, Error> {
    loop {
        read_until_magic_number(data).await?;
        let mut buf = [0u8; 4];
        data.read_exact(&mut buf).await?;
        let header = MessageHeader::new_from_be_bytes(&buf).await;
        if header.command_type().is_some() {
            return Ok(header);
        }
        warn!(
            "Read magic number but command type was invalid: {:#04x}",
            header.message_type
        );
    }
}

#[tokio::test]
async fn test_read_until_magic_number_give_magic_number() {
    let mut test: &[u8] = &MAGIC_NUMBER.clone();
    let test_reader: &mut (dyn AsyncRead + Send + Unpin) = &mut test;
    assert!(read_until_magic_number(test_reader).await.is_ok());
}

#[tokio::test]
async fn test_read_until_magic_number_dont_give_magic_number() {
    let mut test: &[u8] = &[0; 10].clone();
    let test_reader: &mut (dyn AsyncRead + Send + Unpin) = &mut test;
    assert!(read_until_magic_number(test_reader).await.is_err());
}

#[tokio::test]
async fn test_read_until_magic_number_hard() {
    let mut test: &[u8] = &[0x32, 0x61, 0x74, 0x64, 0x61, 0x74, 0x64, 0x64, 0x18];
    let test_reader: &mut (dyn AsyncRead + Send + Unpin) = &mut test;
    assert!(read_until_magic_number(test_reader).await.is_ok());
}
