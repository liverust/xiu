use byteorder::BigEndian;
use bytesio::bits_reader::BitsReader;
use bytesio::bytes_errors::BytesReadError;
use bytesio::bytes_errors::BytesWriteError;
use bytesio::bytes_reader::BytesReader;
use bytesio::bytes_writer::BytesWriter;

//  0                   1                   2                   3
//  0 1 2 3 4 5 6 7 8 9 0 1 2 3 4 5 6 7 8 9 0 1 2 3 4 5 6 7 8 9 0 1
// +-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
// |V=2|P|    RC   |   PT          |             length            |
// +-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
#[derive(Debug, Clone, Default)]
pub struct RtcpHeader {
    pub version: u8,      // 2 bits
    pub padding_flag: u8, // 1 bit
    pub report_count: u8, // 5 bit
    pub payload_type: u8, // 8 bit
    pub length: u16,      // 16 bits
}

impl RtcpHeader {
    pub fn unpack(&mut self, reader: &mut BytesReader) -> Result<(), BytesReadError> {
        let byte_1st: u8 = reader.read_u8()?;
        self.version = byte_1st >> 6;
        self.padding_flag = (byte_1st >> 5) & 0x01;
        self.report_count = byte_1st & 0x1F;
        self.payload_type = reader.read_u8()?;
        self.length = reader.read_u16::<BigEndian>()?;

        Ok(())
    }

    pub fn pack(&mut self, writer: &mut BytesWriter) -> Result<(), BytesWriteError> {
        let byte_1st: u8 =
            (self.version << 6) | (self.padding_flag << 5) | (self.report_count << 3);

        writer.write_u8(byte_1st)?;
        writer.write_u8(self.payload_type)?;
        writer.write_u16::<BigEndian>(self.length)?;

        Ok(())
    }
}
