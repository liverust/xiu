pub mod rtp_header;

use bytes::BytesMut;
use bytesio::bytes_errors::BytesReadError;
use bytesio::bytes_reader::BytesReader;
use rtp_header::RtpHeader;

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

        if self.header.extension_flag == 1 {}

        if self.header.padding_flag == 1 {}

        Ok(())
    }
}
