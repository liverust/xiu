use super::RtpPacket;
use bytesio::bytes_errors::BytesReadError;
use bytesio::bytes_reader::BytesReader;
use bytesio::bytes_writer::BytesWriter;
pub struct RtpH264UnPacker {
    writer: BytesWriter,
}

impl RtpH264UnPacker {
    fn unpack(&mut self, reader: &mut BytesReader) -> Result<(), BytesReadError> {
        let mut rtp_packet = RtpPacket::default();

        rtp_packet.unpack(reader)?;

        if let Some(packet_type) = rtp_packet.payload.get(0) {
            match packet_type {
                
            }
        }

        Ok(())
    }
}
