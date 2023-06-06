pub mod define;
pub mod errors;
pub mod rtp_h264;
pub mod rtp_h265;
pub mod rtp_header;
pub mod utils;

use byteorder::BigEndian;
use bytes::{BufMut, BytesMut};
use bytesio::bytes_errors::BytesReadError;
use bytesio::bytes_errors::BytesWriteError;
use bytesio::bytes_reader::BytesReader;
use bytesio::bytes_writer::BytesWriter;
use rtp_header::RtpHeader;

#[derive(Debug, Clone, Default)]
pub struct RtpPacket {
    pub header: RtpHeader,
    pub header_extension_profile: u16,
    pub header_extension_length: u16,
    pub header_extension_payload: BytesMut,
    pub payload: BytesMut,
    pub padding: BytesMut,
}

impl RtpPacket {
    fn new(header: RtpHeader) -> Self {
        Self {
            header,
            ..Default::default()
        }
    }
    //https://blog.jianchihu.net/webrtc-research-rtp-header-extension.html
    pub fn unpack(&mut self, reader: &mut BytesReader) -> Result<(), BytesReadError> {
        self.header.unpack(reader)?;

        if self.header.extension_flag == 1 {
            // 0                   1                   2                   3
            // 0 1 2 3 4 5 6 7 8 9 0 1 2 3 4 5 6 7 8 9 0 1 2 3 4 5 6 7 8 9 0 1
            // +-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
            // |      defined by profile       |           length              |
            // +-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
            // |                        header extension                       |
            // |                             ....                              |
            // header_extension = profile(2 bytes) + length(2 bytes) + header extension payload
            self.header_extension_profile = reader.read_u16::<BigEndian>()?;
            self.header_extension_length = reader.read_u16::<BigEndian>()?;
            self.header_extension_payload =
                reader.read_bytes(4 * self.header_extension_length as usize)?;
        }

        if self.header.padding_flag == 1 {
            let padding_length = reader.get(reader.len() - 1)? as usize;
            self.payload
                .put(reader.read_bytes(reader.len() - padding_length)?);
            self.padding.put(reader.extract_remaining_bytes());
        }

        Ok(())
    }
    pub fn pack(&mut self) -> Result<BytesMut, BytesWriteError> {
        let mut writer = BytesWriter::new();

        self.header.pack(&mut writer)?;

        writer.write_u16::<BigEndian>(self.header_extension_profile)?;
        writer.write_u16::<BigEndian>(self.header_extension_length)?;
        writer.write(&self.header_extension_payload[..])?;

        writer.write(&self.payload[..])?;
        writer.write(&self.padding[..])?;

        Ok(writer.extract_current_bytes())
    }
}
