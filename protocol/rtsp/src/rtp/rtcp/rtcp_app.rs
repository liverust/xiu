use super::errors::RtcpError;
use super::rtcp_header::RtcpHeader;
use super::Marshal;
use super::Unmarshal;
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
#[derive(Debug, Clone, Default)]
pub struct RtcpApp {
    header: RtcpHeader,
    ssrc: u32,
    name: BytesMut,
    app_data: BytesMut,
}

impl Unmarshal<BytesMut, RtcpError> for RtcpApp {
    fn unmarshal(data: BytesMut) -> Result<Self, RtcpError>
    where
        Self: Sized,
    {
        let mut rtcp_app = RtcpApp::default();
        let mut reader = BytesReader::new(data);
        rtcp_app.header = RtcpHeader::unmarshal(&mut reader)?;

        rtcp_app.ssrc = reader.read_u32::<BigEndian>()?;
        rtcp_app.name = reader.read_bytes(4)?;
        rtcp_app.app_data = reader.read_bytes(rtcp_app.header.length as usize * 4)?;

        Ok(rtcp_app)
    }
}

impl Marshal<RtcpError> for RtcpApp {
    fn marshal(&self) -> Result<BytesMut, RtcpError> {
        let mut writer = BytesWriter::default();

        let header_bytesmut = self.header.marshal()?;
        writer.write(&header_bytesmut[..])?;

        writer.write_u32::<BigEndian>(self.ssrc)?;
        writer.write(&self.name[..])?;
        writer.write(&self.app_data[..])?;

        Ok(writer.extract_current_bytes())
    }
}
