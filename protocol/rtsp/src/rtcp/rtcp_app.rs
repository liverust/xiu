use super::rtcp_header::RtcpHeader;
use byteorder::BigEndian;
use bytes::{BufMut, BytesMut};
use bytesio::bytes_errors::BytesReadError;
use bytesio::bytes_errors::BytesWriteError;
use bytesio::bytes_reader::BytesReader;
use bytesio::bytes_writer::BytesWriter;

//  0                   1                   2                   3
//  0 1 2 3 4 5 6 7 8 9 0 1 2 3 4 5 6 7 8 9 0 1 2 3 4 5 6 7 8 9 0 1
// +-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
// |V=2|P|    ST   |   PT=APP=204  |             length            |
// +-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
// |                           SSRC/CSRC                           |
// +-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
// |                          name (ASCII)                         |
// +-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
// |                   application-dependent data                ...
// +-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
pub struct RtcpApp {
    header: RtcpHeader,
    ssrc: u32,
    name: BytesMut,
    app_data: BytesMut,
}

impl RtcpApp {
    pub fn unpack(&mut self, data: BytesMut) -> Result<(), BytesReadError> {
        let mut reader = BytesReader::new(data);
        self.header.unpack(&mut reader)?;

        self.ssrc = reader.read_u32::<BigEndian>()?;
        self.name = reader.read_bytes(4)?;
        self.app_data = reader.read_bytes(self.header.length as usize * 4)?;

        Ok(())
    }

    pub fn pack(&mut self, writer: &mut BytesWriter) -> Result<(), BytesWriteError> {
        self.header.pack(writer)?;

        writer.write_u32::<BigEndian>(self.ssrc)?;
        writer.write(&self.name[..])?;
        writer.write(&self.app_data[..])?;
        Ok(())
    }
}
