use bytes::{BufMut, BytesMut};
use serde_json::{from_str, to_string};
use tokio_util::codec::{Decoder, Encoder};

use crate::message::{ClientMessage, OutgoingMessage};

pub enum FrameError {
    ParseError(serde_json::Error),
    ReadError(std::io::Error),
}

impl From<std::io::Error> for FrameError {
    fn from(value: std::io::Error) -> Self {
        FrameError::ReadError(value)
    }
}

pub struct FrameCodec {}

impl Encoder<OutgoingMessage> for FrameCodec {
    type Error = FrameError;
    fn encode(&mut self, item: OutgoingMessage, dst: &mut BytesMut) -> Result<(), Self::Error> {
        let str = to_string(&item).unwrap();
        dst.put_slice(&str.as_bytes());
        Ok(())
    }
}

impl Decoder for FrameCodec {
    type Error = FrameError;
    type Item = ClientMessage;
    fn decode(&mut self, src: &mut BytesMut) -> Result<Option<Self::Item>, Self::Error> {
        if let Some(pos) = src.iter().position(|&c| c == b'\n') {
            let line = src.split_to(pos + 1);
            let str = String::from_utf8_lossy(&line);
            match from_str::<ClientMessage>(&str) {
                Ok(message) => Ok(Some(message)),
                Err(err) => Err(FrameError::ParseError(err)),
            }
        } else {
            Ok(None)
        }
    }
}
