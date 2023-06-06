use std::ptr::NonNull;

use super::define;
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

pub type OnPacketFn = fn(BytesMut) -> Result<(), RtpH264PackerError>;

pub struct RtpH265Packer {
    header: RtpHeader,
    mtu: usize,
    on_packet_handler: Option<OnPacketFn>,
}

impl RtpH265Packer {
    pub fn pack() {}
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

    +---------------+
    |0|1|2|3|4|5|6|7|
    +-+-+-+-+-+-+-+-+
    |S|E|   FuType  |
    +---------------+
    */
    fn unpack_fu(&mut self, rtp_payload: BytesMut) -> Result<Option<BytesMut>, BytesReadError> {
        let mut payload_reader = BytesReader::new(rtp_payload);
        let payload_header_1st_part = payload_reader.read_u8()?;
        let payload_header_2nd_part = payload_reader.read_u8()?;
        let fu_header = payload_reader.read_u8()?;
        if self.using_donl_field {
            payload_reader.read_bytes(2)?;
        }

        if Self::is_fu_start(fu_header) {
            /*set NAL UNIT type 2 bytes */
            //replace Type of PayloadHdr with the FuType of FU header
            let nal_1st_byte = (payload_header_1st_part & 0x81) | ((fu_header & 0x3F) << 1);
            self.fu_buffer.put_u8(nal_1st_byte);
            self.fu_buffer.put_u8(payload_header_2nd_part);
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
