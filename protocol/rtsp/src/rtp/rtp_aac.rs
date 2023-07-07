use std::fmt::Debug;

use super::define;
use super::errors::PackerError;
use super::errors::UnPackerError;
use super::utils;
use super::utils::Marshal;
use super::utils::OnFrameFn;
use super::utils::TPacker;
use super::utils::TRtpPacker;
use super::utils::TUnPacker;
use super::utils::Unmarshal;
use super::RtpHeader;
use super::RtpPacket;
use byteorder::BigEndian;
use bytes::{BufMut, BytesMut};
use bytesio::bytes_errors::BytesReadError;
use bytesio::bytes_reader::BytesReader;
use bytesio::bytes_writer::BytesWriter;
use streamhub::define::FrameData;

pub type OnPacketFn = fn(BytesMut) -> Result<(), PackerError>;

#[derive(Debug, Clone, Default)]
pub struct RtpAacPacker {
    header: RtpHeader,
    mtu: usize,
    on_packet_handler: Option<OnPacketFn>,
}

impl RtpAacPacker {
    pub fn new(payload_type: u8, ssrc: u32, init_seq: u16, mtu: usize) -> Self {
        RtpAacPacker {
            header: RtpHeader {
                payload_type,
                seq_number: init_seq,
                ssrc,
                ..Default::default()
            },
            mtu,
            ..Default::default()
        }
    }
}

impl TPacker for RtpAacPacker {
    fn pack(&mut self, data: &mut BytesMut) -> Result<(), PackerError> {
        let data_len = data.len();

        let mut packet = RtpPacket::new(self.header.clone());
        packet.payload.put_u16(16);
        packet.payload.put_u8((data_len >> 5) as u8);
        packet.payload.put_u8(((data_len & 0x1F) << 3) as u8);
        packet.payload.put(data);

        let packet_data = packet.marshal()?;
        if let Some(f) = self.on_packet_handler {
            f(packet_data)?;
        }

        self.header.seq_number += 1;

        Ok(())
    }
}

#[derive(Default)]
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
    pub fn new() -> Self {
        RtpAacUnPacker::default()
    }
}

impl TUnPacker for RtpAacUnPacker {
    fn unpack(&mut self, reader: &mut BytesReader) -> Result<(), UnPackerError> {
        let mut rtp_packet = RtpPacket::unmarshal(reader)?;

        let au_headers_length = (reader.read_u16::<BigEndian>()? + 7) / 8;
        let au_header_length = 2;
        let aus_number = au_headers_length / au_header_length;

        let mut au_lengths = Vec::new();
        for _ in 0..aus_number {
            let au_length =
                (((reader.read_u8()? as u16) << 8) | ((reader.read_u8()? as u16) & 0xF8)) as usize;
            au_lengths.push(au_length / 8);
        }

        for au_length in au_lengths {
            let au_data = reader.read_bytes(au_length)?;
            if let Some(f) = &self.on_frame_handler {
                f(FrameData::Video {
                    timestamp: self.timestamp,
                    data: au_data,
                });
            }
        }

        Ok(())
    }
    fn on_frame_handler(&mut self, f: OnFrameFn) {
        self.on_frame_handler = Some(f);
    }
}
