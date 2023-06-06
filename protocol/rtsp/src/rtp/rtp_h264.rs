use super::define;
use super::errors::RtpH264PackerError;
use super::utils;
use super::RtpHeader;
use super::RtpPacket;
use byteorder::BigEndian;
use bytes::{BufMut, BytesMut};
use bytesio::bytes_errors::BytesReadError;
use bytesio::bytes_reader::BytesReader;
use bytesio::bytes_writer::BytesWriter;

pub type OnPacketFn = fn(BytesMut) -> Result<(), RtpH264PackerError>;

pub struct RtpH264Packer {
    header: RtpHeader,
    mtu: usize,
    on_packet_handler: Option<OnPacketFn>,
}

impl RtpH264Packer {
    //pack annexb h264 data
    pub fn pack(&mut self, nalus: &mut BytesMut) -> Result<(), RtpH264PackerError> {
        while nalus.len() > 0 {
            if let Some(pos_left) = utils::find_start_code(&nalus[..]) {
                let mut nalu_with_start_code =
                    if let Some(pos_right) = utils::find_start_code(&nalus[pos_left + 3..]) {
                        nalus.split_to(pos_left + pos_right + 3)
                    } else {
                        nalus.split_to(nalus.len())
                    };

                let nalu = nalu_with_start_code.split_off(pos_left + 3);
                if nalu.len() + RTP_FIXED_HEADER_LEN <= self.mtu {
                    return self.pack_single(nalu);
                } else {
                    return self.pack_fu_a(nalu);
                }
            } else {
                break;
            }
        }

        Ok(())
    }
    pub fn pack_fu_a(&mut self, nalu: BytesMut) -> Result<(), RtpH264PackerError> {
        let mut nalu_reader = BytesReader::new(nalu);
        let byte_1st = nalu_reader.read_u8()?;

        let fu_indicator: u8 = (byte_1st & 0xE0) | FU_A;
        let mut fu_header: u8 = (byte_1st & 0x1F) | FU_START;

        let mut left_nalu_bytes: usize = nalu_reader.len();
        let mut fu_payload_len;

        while left_nalu_bytes > 0 {
            if left_nalu_bytes + RTP_FIXED_HEADER_LEN <= self.mtu - 2 {
                fu_header = (byte_1st & 0x1F) | FU_END;
                fu_payload_len = left_nalu_bytes;
            } else {
                fu_payload_len = self.mtu - RTP_FIXED_HEADER_LEN - 2;
            }

            let mut packet = RtpPacket::new(self.header.clone());
            packet.payload.put_u8(fu_indicator);
            packet.payload.put_u8(fu_header);
            let fu_payload = nalu_reader.read_bytes(fu_payload_len)?;
            packet.payload.put(fu_payload);
            packet.header.marker = if fu_header & FU_END > 0 { 1 } else { 0 };

            let packet_bytesmut = packet.pack()?;
            if let Some(f) = self.on_packet_handler {
                f(packet_bytesmut);
            }

            left_nalu_bytes = nalu_reader.len();
            self.header.seq_number += 1;
        }

        Ok(())
    }
    pub fn pack_single(&mut self, nalu: BytesMut) -> Result<(), RtpH264PackerError> {
        let mut packet = RtpPacket::new(self.header.clone());
        packet.header.marker = 1;
        packet.payload.put(nalu);

        let packet_bytesmut = packet.pack()?;

        if let Some(f) = self.on_packet_handler {
            f(packet_bytesmut);
        }

        Ok(())
    }
}

pub struct RtpH264UnPacker {
    sequence_number: u16,
    timestamp: u32,
    fu_buffer: BytesMut,
    flags: i16,
}

type RtpNalType = u8;

pub const STAP_A: RtpNalType = 24;
pub const STAP_B: RtpNalType = 25;
pub const MTAP_16: RtpNalType = 26;
pub const MTAP_24: RtpNalType = 27;
pub const FU_A: RtpNalType = 28;
pub const FU_B: RtpNalType = 29;
pub const FU_START: u8 = 0x80;
pub const FU_END: u8 = 0x40;
pub const RTP_FIXED_HEADER_LEN: usize = 12;

impl RtpH264UnPacker {
    pub fn unpack(&mut self, reader: &mut BytesReader) -> Result<Option<BytesMut>, BytesReadError> {
        let mut rtp_packet = RtpPacket::default();
        rtp_packet.unpack(reader)?;

        if let Some(packet_type) = rtp_packet.payload.get(0) {
            match *packet_type & 0x1F {
                1..=23 => {
                    return self.unpack_single(rtp_packet.payload.clone(), *packet_type);
                }
                STAP_A | STAP_B => {
                    return self.unpack_stap(rtp_packet.payload.clone(), *packet_type);
                }
                MTAP_16 | MTAP_24 => {
                    return self.unpack_mtap(rtp_packet.payload.clone(), *packet_type);
                }
                FU_A | FU_B => {
                    return self.unpack_fu(rtp_packet.payload.clone(), *packet_type);
                }
                _ => {}
            }
        }

        Ok(None)
    }

    fn unpack_single(
        &mut self,
        rtp_payload: BytesMut,
        t: RtpNalType,
    ) -> Result<Option<BytesMut>, BytesReadError> {
        return Ok(Some(rtp_payload));
    }

    //  0 1 2 3 4 5 6 7 8 9 0 1 2 3 4 5 6 7 8 9 0 1 2 3 4 5 6 7 8 9 0 1
    // +-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
    // | FU indicator  |   FU header   |                               |
    // +-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+                               |
    // |                                                               |
    // |                         FU payload                            |
    // |                                                               |
    // |                               +-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
    // |                               :...OPTIONAL RTP padding        |
    // +-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+

    //   RTP payload format for FU-A

    //  0                   1                   2                   3
    //  0 1 2 3 4 5 6 7 8 9 0 1 2 3 4 5 6 7 8 9 0 1 2 3 4 5 6 7 8 9 0 1
    // +-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
    // | FU indicator  |   FU header   |               DON             |
    // +-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-|
    // |                                                               |
    // |                         FU payload                            |
    // |                                                               |
    // |                               +-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
    // |                               :...OPTIONAL RTP padding        |
    // +-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+

    //   RTP payload format for FU-B

    // FU indicator
    // +---------------+
    // |0|1|2|3|4|5|6|7|
    // +-+-+-+-+-+-+-+-+
    // |F|NRI|  Type   |
    // +---------------+

    // FU header
    // +---------------+
    // |0|1|2|3|4|5|6|7|
    // +-+-+-+-+-+-+-+-+
    // |S|E|R|  Type   |
    // +---------------+
    fn unpack_fu(
        &mut self,
        rtp_payload: BytesMut,
        t: RtpNalType,
    ) -> Result<Option<BytesMut>, BytesReadError> {
        let mut payload_reader = BytesReader::new(rtp_payload);
        let fu_indicator = payload_reader.read_u8()?;
        let fu_header = payload_reader.read_u8()?;

        if t == FU_B {
            //read DON
            payload_reader.read_u16::<BigEndian>()?;
        }

        if Self::is_fu_start(fu_header) {
            self.fu_buffer
                .put_u8((fu_indicator & 0xE0) | (fu_header & 0x1F))
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

    //  0                   1                   2                   3
    //  0 1 2 3 4 5 6 7 8 9 0 1 2 3 4 5 6 7 8 9 0 1 2 3 4 5 6 7 8 9 0 1
    // +-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
    // |                          RTP Header                           |
    // +-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
    // |STAP-A NAL HDR |         NALU 1 Size           | NALU 1 HDR    |
    // +-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
    // |                         NALU 1 Data                           |
    // :                                                               :
    // +               +-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
    // |               | NALU 2 Size                   | NALU 2 HDR    |
    // +-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
    // |                         NALU 2 Data                           |
    // :                                                               :
    // |                               +-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
    // |                               :...OPTIONAL RTP padding        |
    // +-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+

    //   An example of an RTP packet including an STAP-A
    //   containing two single-time aggregation units

    //  0                   1                   2                   3
    //  0 1 2 3 4 5 6 7 8 9 0 1 2 3 4 5 6 7 8 9 0 1 2 3 4 5 6 7 8 9 0 1
    // +-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
    // |                          RTP Header                           |
    // +-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
    // |STAP-B NAL HDR | DON                           | NALU 1 Size   |
    // +-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
    // | NALU 1 Size   | NALU 1 HDR    | NALU 1 Data                   |
    // +-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+                               +
    // :                                                               :
    // +               +-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
    // |               | NALU 2 Size                   | NALU 2 HDR    |
    // +-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
    // |                       NALU 2 Data                             |
    // :                                                               :
    // |                               +-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
    // |                               :...OPTIONAL RTP padding        |
    // +-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+

    //   An example of an RTP packet including an STAP-B
    //   containing two single-time aggregation units

    fn unpack_stap(
        &mut self,
        rtp_payload: BytesMut,
        t: RtpNalType,
    ) -> Result<Option<BytesMut>, BytesReadError> {
        let mut payload_reader = BytesReader::new(rtp_payload);
        //STAP-A / STAP-B HDR
        payload_reader.read_u8()?;

        if t == STAP_B {
            //read DON
            payload_reader.read_u16::<BigEndian>()?;
        }
        let mut nalus = BytesMut::new();
        while payload_reader.len() > 0 {
            let length = payload_reader.read_u16::<BigEndian>()? as usize;
            let nalu = payload_reader.read_bytes(length)?;
            nalus.extend_from_slice(&define::ANNEXB_NALU_START_CODE);
            nalus.put(nalu);
        }
        Ok(Some(nalus))
    }

    //  0                   1                   2                   3
    //  0 1 2 3 4 5 6 7 8 9 0 1 2 3 4 5 6 7 8 9 0 1 2 3 4 5 6 7 8 9 0 1
    // +-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
    // |                          RTP Header                           |
    // +-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
    // |MTAP16 NAL HDR |  decoding order number base   | NALU 1 Size   |
    // +-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
    // |  NALU 1 Size  |  NALU 1 DOND  |       NALU 1 TS offset        |
    // +-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
    // |  NALU 1 HDR   |  NALU 1 DATA                                  |
    // +-+-+-+-+-+-+-+-+                                               +
    // :                                                               :
    // +               +-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
    // |               | NALU 2 SIZE                   |  NALU 2 DOND  |
    // +-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
    // |       NALU 2 TS offset        |  NALU 2 HDR   |  NALU 2 DATA  |
    // +-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+               |
    // :                                                               :
    // |                               +-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
    // |                               :...OPTIONAL RTP padding        |
    // +-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+

    //   An RTP packet including a multi-time aggregation
    //   packet of type MTAP16 containing two multi-time
    //   aggregation units

    //  0                   1                   2                   3
    //  0 1 2 3 4 5 6 7 8 9 0 1 2 3 4 5 6 7 8 9 0 1 2 3 4 5 6 7 8 9 0 1
    // +-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
    // |                          RTP Header                           |
    // +-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
    // |MTAP24 NAL HDR |  decoding order number base   | NALU 1 Size   |
    // +-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
    // |  NALU 1 Size  |  NALU 1 DOND  |       NALU 1 TS offs          |
    // +-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
    // |NALU 1 TS offs |  NALU 1 HDR   |  NALU 1 DATA                  |
    // +-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+                               +
    // :                                                               :
    // +               +-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
    // |               | NALU 2 SIZE                   |  NALU 2 DOND  |
    // +-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
    // |       NALU 2 TS offset                        |  NALU 2 HDR   |
    // +-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
    // |  NALU 2 DATA                                                  |
    // :                                                               :
    // |                               +-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
    // |                               :...OPTIONAL RTP padding        |
    // +-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+

    //   An RTP packet including a multi-time aggregation
    //   packet of type MTAP24 containing two multi-time
    //   aggregation units

    fn unpack_mtap(
        &mut self,
        rtp_payload: BytesMut,
        t: RtpNalType,
    ) -> Result<Option<BytesMut>, BytesReadError> {
        let mut payload_reader = BytesReader::new(rtp_payload);
        //read NAL HDR
        payload_reader.read_u8()?;
        //read decoding_order_number_base
        payload_reader.read_u16::<BigEndian>()?;

        let mut nalus = BytesMut::new();
        while payload_reader.len() > 0 {
            //read nalu size
            let nalu_size = payload_reader.read_u16::<BigEndian>()? as usize;
            // read dond
            payload_reader.read_u8()?;
            // read TS offs
            let (ts, ts_bytes) = if t == MTAP_16 {
                (payload_reader.read_u16::<BigEndian>()? as u32, 2_usize)
            } else if t == MTAP_24 {
                (payload_reader.read_u24::<BigEndian>()?, 3_usize)
            } else {
                log::warn!("should not be here!");
                (0, 0)
            };
            assert!(ts != 0);
            let nalu = payload_reader.read_bytes(nalu_size - ts_bytes - 1)?;
            nalus.extend_from_slice(&define::ANNEXB_NALU_START_CODE);
            nalus.put(nalu);
        }

        Ok(Some(nalus))
    }

    fn is_fu_start(fu_header: u8) -> bool {
        fu_header & FU_START > 0
    }

    fn is_fu_end(fu_header: u8) -> bool {
        fu_header & FU_END > 0
    }
}
