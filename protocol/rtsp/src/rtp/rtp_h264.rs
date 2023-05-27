use super::RtpPacket;
use byteorder::BigEndian;
use bytes::{BufMut, BytesMut};
use bytesio::bytes_errors::BytesReadError;
use bytesio::bytes_reader::BytesReader;
use bytesio::bytes_writer::BytesWriter;
pub struct RtpH264UnPacker {
    sequence_number: u16,
    timestamp: u32,
    fu_buffer: BytesMut,
    fu_indicator: u8,
    fu_header: u8,
    flags: i16,
}

type RtpNalType = u8;

pub const STAP_A: RtpNalType = 24;
pub const STAP_B: RtpNalType = 25;
pub const MTAP_16: RtpNalType = 26;
pub const MTAP_24: RtpNalType = 27;
pub const FU_A: RtpNalType = 28;
pub const FU_B: RtpNalType = 29;

const ANNEXB_NALU_START_CODE: [u8; 4] = [0x00, 0x00, 0x00, 0x01];

impl RtpH264UnPacker {
    pub fn unpack(&mut self, reader: &mut BytesReader) -> Result<Option<BytesMut>, BytesReadError> {
        let mut rtp_packet = RtpPacket::default();

        rtp_packet.unpack(reader)?;

        if let Some(packet_type) = rtp_packet.payload.get(0) {
            match *packet_type {
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
        self.fu_indicator = payload_reader.read_u8()?;
        self.fu_header = payload_reader.read_u8()?;

        if t == FU_B {
            //read DON
            payload_reader.read_u16::<BigEndian>()?;
        }

        if Self::is_fu_start(self.fu_header) {}

        self.fu_buffer.put(payload_reader.extract_remaining_bytes());

        if Self::is_fu_end(self.fu_header) {
            let mut packet = BytesMut::new();
            packet.extend_from_slice(&ANNEXB_NALU_START_CODE);
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
            nalus.extend_from_slice(&ANNEXB_NALU_START_CODE);
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
            nalus.extend_from_slice(&ANNEXB_NALU_START_CODE);
            nalus.put(nalu);
        }

        Ok(Some(nalus))
    }

    fn is_fu_start(fu_header: u8) -> bool {
        fu_header & 0x80 > 0
    }

    fn is_fu_end(fu_header: u8) -> bool {
        fu_header & 0x40 > 0
    }
}
