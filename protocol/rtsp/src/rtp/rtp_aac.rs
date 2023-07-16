use super::errors::PackerError;
use super::errors::UnPackerError;

use super::utils::Marshal;
use super::utils::OnFrameFn;
use super::utils::OnPacket2Fn;
use super::utils::OnPacketFn;
use super::utils::TPacker;

use super::utils::TRtpReceiverForRtcp;
use super::utils::TUnPacker;
use super::utils::Unmarshal;
use super::RtpHeader;
use super::RtpPacket;
use async_trait::async_trait;
use byteorder::BigEndian;
use bytes::{BufMut, BytesMut};

use bytesio::bytes_reader::BytesReader;
use bytesio::bytesio::TNetIO;
use std::sync::Arc;
use streamhub::define::FrameData;
use tokio::sync::Mutex;

// pub type OnPacketFn = fn(BytesMut) -> Result<(), PackerError>;

pub struct RtpAacPacker {
    header: RtpHeader,
    mtu: usize,
    on_packet_handler: Option<OnPacketFn>,
    io: Arc<Mutex<Box<dyn TNetIO + Send + Sync>>>,
}

impl RtpAacPacker {
    pub fn new(
        payload_type: u8,
        ssrc: u32,
        init_seq: u16,
        mtu: usize,
        io: Arc<Mutex<Box<dyn TNetIO + Send + Sync>>>,
    ) -> Self {
        RtpAacPacker {
            header: RtpHeader {
                payload_type,
                seq_number: init_seq,
                ssrc,
                version: 2,
                marker: 1,
                ..Default::default()
            },
            mtu,
            io,
            on_packet_handler: None,
        }
    }
}
#[async_trait]
impl TPacker for RtpAacPacker {
    async fn pack(&mut self, data: &mut BytesMut, timestamp: u32) -> Result<(), PackerError> {
        self.header.timestamp = timestamp;

        let data_len = data.len();
        let mut packet = RtpPacket::new(self.header.clone());
        packet.payload.put_u16(16);
        packet.payload.put_u8((data_len >> 5) as u8);
        packet.payload.put_u8(((data_len & 0x1F) << 3) as u8);
        packet.payload.put(data);

        let packet_data = packet.marshal()?;
        if let Some(f) = &self.on_packet_handler {
            f(self.io.clone(), packet_data).await?;
        }

        self.header.seq_number += 1;

        Ok(())
    }

    fn on_packet_handler(&mut self, f: OnPacketFn) {
        self.on_packet_handler = Some(f);
    }
}

impl TRtpReceiverForRtcp for RtpAacPacker {
    fn on_rtp(&mut self, f: OnPacket2Fn) {}
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

        let mut reader_payload = BytesReader::new(rtp_packet.payload);

        let au_headers_length = (reader_payload.read_u16::<BigEndian>()? + 7) / 8;
        let au_header_length = 2;
        let aus_number = au_headers_length / au_header_length;

        let mut au_lengths = Vec::new();
        for _ in 0..aus_number {
            let au_length = (((reader_payload.read_u8()? as u16) << 8)
                | ((reader_payload.read_u8()? as u16) & 0xF8)) as usize;
            au_lengths.push(au_length / 8);
        }

        // log::info!(
        //     "send audio : au_headers_length :{}, aus_number: {}, au_lengths: {:?}",
        //     au_headers_length,
        //     aus_number,
        //     au_lengths,
        // );

        for au_length in au_lengths {
            let au_data = reader_payload.read_bytes(au_length)?;
            if let Some(f) = &self.on_frame_handler {
                f(FrameData::Audio {
                    timestamp: rtp_packet.header.timestamp,
                    data: au_data,
                })?;
            }
        }

        Ok(())
    }
    fn on_frame_handler(&mut self, f: OnFrameFn) {
        self.on_frame_handler = Some(f);
    }
}

impl TRtpReceiverForRtcp for RtpAacUnPacker {
    fn on_rtp(&mut self, f: OnPacket2Fn) {}
}
