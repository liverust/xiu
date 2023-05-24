pub mod rtp_h264;
pub mod rtp_header;

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
    pub header_extension: BytesMut,
    pub payload: BytesMut,
    pub padding: BytesMut,
}

impl RtpPacket {
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
            let profile = reader.read_u16::<BigEndian>()?;
            self.header_extension.put_u16(profile);
            let length = reader.read_u16::<BigEndian>()? as usize;
            let header_extension_payload = reader.read_bytes(4 * length)?;
            self.header_extension.put(header_extension_payload);
        }

        if self.header.padding_flag == 1 {
            let padding_length = reader.get(reader.len() - 1)? as usize;
            self.payload
                .put(reader.read_bytes(reader.len() - padding_length)?);
            self.padding.put(reader.extract_remaining_bytes());
        }

        Ok(())
    }
    pub fn pack(&mut self, writer: &mut BytesWriter) -> Result<(), BytesWriteError> {
        self.header.pack(writer)?;

        writer.write(&self.header_extension[..])?;
        writer.write(&self.payload[..])?;
        writer.write(&self.padding[..])?;

        Ok(())
    }
}
