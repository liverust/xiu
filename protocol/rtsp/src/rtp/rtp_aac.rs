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

pub type OnPacketFn = fn(BytesMut) -> Result<(), RtpH264PackerError>;

pub struct RtpAacPacker {
    header: RtpHeader,
    mtu: usize,
    on_packet_handler: Option<OnPacketFn>,
}

impl RtpAacPacker {
    fn pack(data: &mut BytesMut) -> Result<(), RtpH264PackerError> {
        let mut data_reader = BytesReader::new(data);
        let check_bytes = data_reader.advance_bytes(2)?;
        let byte_0 = check_bytes[0];
        let byte_1 = check_bytes[1];

        if 0xFF == byte_0 && 0xF0 == (byte_1 & 0xF0) && data_reader.len() > 7 {
            data_reader.read_bytes(7)?;
        }

        let mut packet = RtpPacket::new(self.header.clone());

        Ok(())
    }
}

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
