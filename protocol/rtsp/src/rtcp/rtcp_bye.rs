use super::rtcp_header::RtcpHeader;
use byteorder::BigEndian;
use bytes::{BufMut, BytesMut};
use bytesio::bytes_errors::BytesReadError;
use bytesio::bytes_errors::BytesWriteError;
use bytesio::bytes_reader::BytesReader;
use bytesio::bytes_writer::BytesWriter;
//  	  0                   1                   2                   3
//  	  0 1 2 3 4 5 6 7 8 9 0 1 2 3 4 5 6 7 8 9 0 1 2 3 4 5 6 7 8 9 0 1
// 	     +-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
// 	     |V=2|P|    SC   |   PT=BYE=203  |             length            |
// 	     +-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
// 	     |                           SSRC/CSRC                           |
// 	     +-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
// 	     :                              ...                              :
// 	     +=+=+=+=+=+=+=+=+=+=+=+=+=+=+=+=+=+=+=+=+=+=+=+=+=+=+=+=+=+=+=+=+
// (opt) |     length    |            reason for leaving     ...
// 	     +-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
pub struct RtcpBye {
    header: RtcpHeader,
    ssrss: Vec<u32>,
    length: u8,
    reason: BytesMut,
}
impl RtcpBye {
    pub fn unpack(&mut self, data: BytesMut) -> Result<(), BytesReadError> {
        let mut reader = BytesReader::new(data);
        self.header.unpack(&mut reader)?;

        for _ in 0..self.header.report_count {
            let ssrc = reader.read_u32::<BigEndian>()?;
            self.ssrss.push(ssrc);
        }

        self.length = reader.read_u8()?;
        self.reason = reader.read_bytes(self.length as usize)?;

        Ok(())
    }

    pub fn pack(&mut self, writer: &mut BytesWriter) -> Result<(), BytesWriteError> {
        self.header.pack(writer)?;

        for ssrc in &self.ssrss {
            writer.write_u32::<BigEndian>(*ssrc)?;
        }

        writer.write_u8(self.length)?;
        writer.write(&self.reason[..])?;
        Ok(())
    }
}
