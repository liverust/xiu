use std::ptr::NonNull;

use super::define;
use super::errors::RtpH265PackerError;

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

type RtpNalType = u8;
pub const AP: RtpNalType = 48; //Aggregation Packets
pub const FU: RtpNalType = 49; //Fragmentation Units
pub const PACI: RtpNalType = 50;

pub const FU_START: u8 = 0x80;
pub const FU_END: u8 = 0x40;
pub const RTP_FIXED_HEADER_LEN: usize = 12;

pub type OnPacketFn = fn(BytesMut) -> Result<(), RtpH265PackerError>;

pub struct RtpH265Packer {
    header: RtpHeader,
    mtu: usize,
    on_packet_handler: Option<OnPacketFn>,
}

impl RtpH265Packer {
    pub fn pack_fu(&mut self, nalu: BytesMut) -> Result<(), RtpH265PackerError> {
        let mut nalu_reader = BytesReader::new(nalu);
        /* NALU header
        0               1
        0 1 2 3 4 5 6 7 8 9 0 1 2 3 4 5
        +-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
        |F|    Type   |  LayerId  | TID |
        +-------------+-----------------+

        Forbidden zero(F) : 1 bit
        NAL unit type(Type) : 6 bits
        NUH layer ID(LayerId) : 6 bits
        NUH temporal ID plus 1 (TID) : 3 bits
        */
        let nalu_header_1st_byte = nalu_reader.read_u8()?;
        let nalu_header_2nd_byte = nalu_reader.read_u8()?;

        /* The PayloadHdr needs replace Type with the FU type value(49) */
        let payload_hdr: u16 = ((nalu_header_1st_byte as u16 & 0x81) | ((FU as u16) << 1)) << 8
            | nalu_header_2nd_byte as u16;
        /* FU header
        +---------------+
        |0|1|2|3|4|5|6|7|
        +-+-+-+-+-+-+-+-+
        |S|E|   FuType  |
        +---------------+
        */
        /*set FuType from NALU header's Type */
        let mut fu_header = (nalu_header_1st_byte >> 1) & 0x3F | FU_START;

        let mut left_nalu_bytes: usize = nalu_reader.len();
        let mut fu_payload_len: usize;

        while left_nalu_bytes > 0 {
            /* 3 = PayloadHdr(2 bytes) + FU header(1 byte) */
            if left_nalu_bytes + RTP_FIXED_HEADER_LEN <= self.mtu - 3 {
                fu_header = (nalu_header_1st_byte & 0x1F) | FU_END;
                fu_payload_len = left_nalu_bytes;
            } else {
                fu_payload_len = self.mtu - RTP_FIXED_HEADER_LEN - 3;
            }

            let fu_payload = nalu_reader.read_bytes(fu_payload_len)?;

            let mut packet = RtpPacket::new(self.header.clone());
            packet.payload.put_u16(payload_hdr);
            packet.payload.put_u8(fu_header);
            packet.payload.put(fu_payload);
            packet.header.marker = if fu_header & FU_END > 0 { 1 } else { 0 };

            let packet_bytesmut = packet.pack()?;
            if let Some(f) = self.on_packet_handler {
                f(packet_bytesmut)?;
            }
            left_nalu_bytes = nalu_reader.len();
            self.header.seq_number += 1;
        }

        Ok(())
    }
    pub fn pack_single(&mut self, nalu: BytesMut) -> Result<(), RtpH265PackerError> {
        let mut packet = RtpPacket::new(self.header.clone());
        packet.header.marker = 1;
        packet.payload.put(nalu);

        let packet_bytesmut = packet.pack()?;
        self.header.seq_number += 1;

        if let Some(f) = self.on_packet_handler {
            return f(packet_bytesmut);
        }
        Ok(())
    }
}
impl TRtpPacker for RtpH265Packer {
    fn pack(&mut self, nalus: &mut BytesMut) -> Result<(), RtpPackerError> {
        utils::split_annexb_and_process(nalus, self)?;
        Ok(())
    }

    fn pack_nalu(&mut self, nalu: BytesMut) -> Result<(), RtpPackerError> {
        if nalu.len() + RTP_FIXED_HEADER_LEN <= self.mtu {
            self.pack_single(nalu)?;
        } else {
            self.pack_fu(nalu)?;
        }
        Ok(())
    }
}

pub struct RtpH265UnPacker {
    sequence_number: u16,
    timestamp: u32,
    fu_buffer: BytesMut,
    flags: i16,
    using_donl_field: bool,
}

impl RtpH265UnPacker {
    pub fn unpack(&mut self, reader: &mut BytesReader) -> Result<Option<BytesMut>, BytesReadError> {
        let mut rtp_packet = RtpPacket::default();
        rtp_packet.unpack(reader)?;

        if let Some(packet_type) = rtp_packet.payload.get(0) {
            match *packet_type >> 1 & 0x3F {
                1..=39 => {
                    return self.unpack_single(rtp_packet.payload.clone());
                }
                FU => {
                    return self.unpack_fu(rtp_packet.payload.clone());
                }
                AP => {
                    return self.unpack_ap(rtp_packet.payload);
                }
                PACI.. => {}

                _ => {}
            }
        }

        Ok(None)
    }

    fn unpack_single(&mut self, rtp_payload: BytesMut) -> Result<Option<BytesMut>, BytesReadError> {
        return Ok(Some(rtp_payload));
    }

    /*
     0               1               2               3
     0 1 2 3 4 5 6 7 8 9 0 1 2 3 4 5 6 7 8 9 0 1 2 3 4 5 6 7 8 9 0 1
    +-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
    |                          RTP Header                           |
    +-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
    |      PayloadHdr (Type=48)     |           NALU 1 DONL         |
    +-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
    |           NALU 1 Size         |            NALU 1 HDR         |
    +-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
    |                                                               |
    |                         NALU 1 Data . . .                     |
    |                                                               |
    +     . . .     +-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
    |               |  NALU 2 DOND  |            NALU 2 Size        |
    +-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
    |          NALU 2 HDR           |                               |
    +-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+            NALU 2 Data        |
    |                                                               |
    |         . . .                 +-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
    |                               :    ...OPTIONAL RTP padding    |
    +-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
    */

    fn unpack_ap(&mut self, rtp_payload: BytesMut) -> Result<Option<BytesMut>, BytesReadError> {
        let mut payload_reader = BytesReader::new(rtp_payload);
        /*read PayloadHdr*/
        payload_reader.read_bytes(2)?;

        let mut nalus = BytesMut::new();
        while payload_reader.len() > 0 {
            if self.using_donl_field {
                /*read DONL*/
                payload_reader.read_bytes(2)?;
            }
            /*read NALU Size*/
            let nalu_len = payload_reader.read_u16::<BigEndian>()? as usize;
            /*read NALU HDR + Data */
            let nalu = payload_reader.read_bytes(nalu_len)?;

            nalus.extend_from_slice(&define::ANNEXB_NALU_START_CODE);
            nalus.put(nalu);
        }

        Ok(Some(nalus))
    }

    /*
    0               1
    0 1 2 3 4 5 6 7 8 9 0 1 2 3 4 5
    +-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
    |F|    Type   |  LayerId  | TID |
    +-------------+-----------------+

    Forbidden zero(F) : 1 bit
    NAL unit type(Type) : 6 bits
    NUH layer ID(LayerId) : 6 bits
    NUH temporal ID plus 1 (TID) : 3 bits
    */

    /*
     0               1               2               3
     0 1 2 3 4 5 6 7 8 9 0 1 2 3 4 5 6 7 8 9 0 1 2 3 4 5 6 7 8 9 0 1
    +-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
    |     PayloadHdr (Type=49)      |    FU header  |  DONL (cond)  |
    +-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-|
    |  DONL (cond)  |                                               |
    |-+-+-+-+-+-+-+-+                                               |
    |                           FU payload                          |
    |                                                               |
    |                               +-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
    |                               :    ...OPTIONAL RTP padding    |
    +-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
    /* FU header */
    +---------------+
    |0|1|2|3|4|5|6|7|
    +-+-+-+-+-+-+-+-+
    |S|E|   FuType  |
    +---------------+
    */
    fn unpack_fu(&mut self, rtp_payload: BytesMut) -> Result<Option<BytesMut>, BytesReadError> {
        let mut payload_reader = BytesReader::new(rtp_payload);
        let payload_header_1st_byte = payload_reader.read_u8()?;
        let payload_header_2nd_byte = payload_reader.read_u8()?;
        let fu_header = payload_reader.read_u8()?;
        if self.using_donl_field {
            payload_reader.read_bytes(2)?;
        }

        if Self::is_fu_start(fu_header) {
            /*set NAL UNIT type 2 bytes */
            //replace Type of PayloadHdr with the FuType of FU header
            let nal_1st_byte = (payload_header_1st_byte & 0x81) | ((fu_header & 0x3F) << 1);
            self.fu_buffer.put_u8(nal_1st_byte);
            self.fu_buffer.put_u8(payload_header_2nd_byte);
        }

        self.fu_buffer.put(payload_reader.extract_remaining_bytes());

        if Self::is_fu_end(fu_header) {
            let mut packet = BytesMut::new();
            packet.extend_from_slice(&define::ANNEXB_NALU_START_CODE);
            packet.put(self.fu_buffer.clone());
            self.fu_buffer.clear();
            return Ok(Some(packet));
        }

        Ok(None)
    }

    fn is_fu_start(fu_header: u8) -> bool {
        fu_header & FU_START > 0
    }

    fn is_fu_end(fu_header: u8) -> bool {
        fu_header & FU_END > 0
    }
}
