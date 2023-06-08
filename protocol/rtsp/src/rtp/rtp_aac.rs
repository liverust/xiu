use super::define;
use super::errors::RtpH264PackerError;
use super::errors::RtpPackerError;
use super::utils;
use super::utils::TRtpPacker;
use super::RtpHeader;
use super::RtpPacket;
use byteorder::BigEndian;
use bytes::{BufMut, BytesMut};
use bytesio::bytes_errors::BytesReadError;
use bytesio::bytes_reader::BytesReader;
use bytesio::bytes_writer::BytesWriter;

pub type OnFrameFn = fn(BytesMut) -> Result<(), RtpH264PackerError>;
pub struct RtpAacUnPacker {
    sequence_number: u16,
    timestamp: u32,
    fu_buffer: BytesMut,
    flags: i16,
    on_frame_handler: Option<OnFrameFn>,
}

// +---------+-----------+-----------+---------------+
// | RTP     | AU Header | Auxiliary | Access Unit   |
// | Header  | Section   | Section   | Data Section  |
// +---------+-----------+-----------+---------------+
// 	<----------RTP Packet Payload----------->
//
// +-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+- .. -+-+-+-+-+-+-+-+-+-+
// |AU-headers-length|AU-header|AU-header|      |AU-header|padding|
// |                 |   (1)   |   (2)   |      |   (n)   | bits  |
// +-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+- .. -+-+-+-+-+-+-+-+-+-+

// Au-headers-length 2 bytes

impl RtpAacUnPacker {
    fn unpack(&mut self, reader: &mut BytesReader) -> Result<(), BytesReadError> {
        let mut rtp_packet = RtpPacket::default();
        rtp_packet.unpack(reader)?;
        let au_headers_length = (reader.read_u16::<BigEndian>()? + 7) / 8;
        let au_header_length = 2;
        let aus_number = au_headers_length / au_header_length;

        let mut au_lengths = Vec::new();
        for _ in 0..aus_number {
            let au_length = ((reader.read_u8()? << 8) | (reader.read_u8()? & 0xF8)) as usize;
            au_lengths.push(au_length / 8);
        }

        for au_length in au_lengths {
            let au_data = reader.read_bytes(au_length)?;
            if let Some(f) = self.on_frame_handler {
                f(au_data);
            }
        }

        Ok(())
    }
}
