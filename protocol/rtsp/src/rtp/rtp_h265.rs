use std::ptr::NonNull;

use super::errors::RtpH264PackerError;
use super::RtpHeader;
use super::RtpPacket;
use byteorder::BigEndian;
use bytes::{BufMut, BytesMut};
use bytesio::bytes_errors::BytesReadError;
use bytesio::bytes_reader::BytesReader;
use bytesio::bytes_writer::BytesWriter;

type RtpNalType = u8;
pub const AP: RtpNalType = 48; //Aggregation Packets
pub const FU: RtpNalType = 49; //Fragmentation Units
pub const PACI: RtpNalType = 50;

pub const FU_START: u8 = 0x80;
pub const FU_END: u8 = 0x40;
pub const RTP_FIXED_HEADER_LEN: usize = 12;

const ANNEXB_NALU_START_CODE: [u8; 4] = [0x00, 0x00, 0x00, 0x01];

pub struct RtpH265UnPacker {
    sequence_number: u16,
    timestamp: u32,
    fu_buffer: BytesMut,
    flags: i16,
}

impl RtpH265UnPacker {
    pub fn unpack(&mut self, reader: &mut BytesReader) -> Result<Option<BytesMut>, BytesReadError> {
        let mut rtp_packet = RtpPacket::default();

        rtp_packet.unpack(reader)?;

        if let Some(packet_type) = rtp_packet.payload.get(0) {
            match *packet_type >> 1 & 0x3F {
                1..=39 => return self.unpack_single(rtp_packet.payload.clone()),
                FU => {}
                AP => {}
                PACI.. => {}

                _ => {}
            }
        }

        Ok(None)
    }

    fn unpack_single(&mut self, rtp_payload: BytesMut) -> Result<Option<BytesMut>, BytesReadError> {
        return Ok(Some(rtp_payload));
    }

    fn unpack_fu(&mut self) -> Result<Option<BytesMut>, BytesReadError> {
        Ok(None)
    }
}
