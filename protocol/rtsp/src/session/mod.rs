pub mod define;
pub mod errors;
use crate::rtsp_transport::RtspTransport;
use byteorder::BigEndian;
use bytes::BytesMut;
use bytesio::bytes_reader::AsyncBytesReader;
use errors::SessionError;

use define::rtsp_method_name;
use httparse::Request;
use httparse::Response;
use std::sync::Arc;
use tokio::sync::Mutex;
use uuid::Uuid;

pub struct RtspServerSession {
    reader: AsyncBytesReader,

    bytesio_data: BytesMut,

    transport: RtspTransport,

    pub session_id: Uuid,
}

pub struct InterleavedBinaryData {
    channel_identifier: u8,
    length: u16,
}

impl InterleavedBinaryData {
    // 10.12 Embedded (Interleaved) Binary Data
    // Stream data such as RTP packets is encapsulated by an ASCII dollar
    // sign (24 hexadecimal), followed by a one-byte channel identifier,
    // followed by the length of the encapsulated binary data as a binary,
    // two-byte integer in network byte order
    async fn new(reader: &mut AsyncBytesReader) -> Result<Option<Self>, SessionError> {
        let is_dollar_sign = reader.advance_u8().await? == 0x24;
        if is_dollar_sign {
            reader.read_u8().await?;
            let channel_identifier = reader.read_u8().await?;
            let length = reader.read_u16::<BigEndian>().await?;
            return Ok(Some(InterleavedBinaryData {
                channel_identifier,
                length,
            }));
        }
        Ok(None)
    }
}

impl RtspServerSession {
    async fn run(&mut self) -> Result<(), SessionError> {
        loop {
            if let Ok(data) = InterleavedBinaryData::new(&mut self.reader).await {
                match data {
                    Some(a) => {}
                    None => {}
                }
            }
        }
    }

    async fn on_rtsp_message(&mut self) -> Result<(), SessionError> {
        self.reader.read().await?;

        let data = self.reader.bytes_reader.extract_remaining_bytes();

        let mut headers = [httparse::EMPTY_HEADER; 16];
        let mut req = Request::new(&mut headers);
        let res = req.parse(&data[..]).unwrap();
        if res.is_partial() {
            match req.path {
                Some(ref path) => {
                    // check router for path.
                    // /404 doesn't exist? we could stop parsing
                }
                None => {
                    // must read more and parse again
                }
            }
        } else if let Some(method) = req.method {
            match method {
                //OPTIONS rtsp://127.0.0.1:5544/stream RTSP/1.0
                //CSeq: 1
                //User-Agent: Lavf58.76.100
                rtsp_method_name::OPTIONS => {}
                rtsp_method_name::DESCRIBE => {}
                rtsp_method_name::ANNOUNCE => {}
                rtsp_method_name::SETUP => {}
                rtsp_method_name::PLAY => {}
                rtsp_method_name::PAUSE => {}
                rtsp_method_name::TEARDOWN => {}
                rtsp_method_name::GET_PARAMETER => {}
                rtsp_method_name::SET_PARAMETER => {}
                rtsp_method_name::REDIRECT => {}
                rtsp_method_name::RECORD => {}
                _ => {}
            }
        }

        Ok(())
    }

    fn on_rtp_over_rtsp_message(&mut self) {}
    fn send_response(&mut self) {}
}
