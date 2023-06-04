use byteorder::BigEndian;
use bytesio::bits_reader::BitsReader;
use bytesio::bytes_errors::BytesReadError;
use bytesio::bytes_errors::BytesWriteError;
use bytesio::bytes_reader::BytesReader;
use bytesio::bytes_writer::BytesWriter;

#[derive(Debug, Clone, Default)]
pub struct RtpHeader {
    pub version: u8,        // 2 bits
    pub padding_flag: u8,   // 1 bit
    pub extension_flag: u8, // 1 bit
    pub cc: u8,             // 4 bits
    pub marker: u8,         // 1 bit
    pub payload_type: u8,   // 7 bits
    pub seq_number: u16,
    pub timestamp: u32,
    pub ssrc: u32,
    pub csrcs: Vec<u32>,
}

impl RtpHeader {
    pub fn unpack(&mut self, reader: &mut BytesReader) -> Result<(), BytesReadError> {
        let byte_1st: u8 = reader.read_u8()?;
        self.version = byte_1st >> 6;
        self.padding_flag = byte_1st >> 5 & 0x01;
        self.extension_flag = byte_1st >> 4 & 0x01;
        self.cc = byte_1st & 0x0F;

        let byte_2nd = reader.read_u8()?;
        self.marker = byte_2nd >> 7;
        self.payload_type = byte_2nd & 0x7F;
        self.seq_number = reader.read_u16::<BigEndian>()?;
        self.timestamp = reader.read_u32::<BigEndian>()?;
        self.ssrc = reader.read_u32::<BigEndian>()?;

        for _ in 0..self.cc {
            self.csrcs.push(reader.read_u32::<BigEndian>()?);
        }

        Ok(())
    }

    //  0                   1                   2                   3
    //  0 1 2 3 4 5 6 7 8 9 0 1 2 3 4 5 6 7 8 9 0 1 2 3 4 5 6 7 8 9 0 1
    // +-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
    // |V=2|P|X|  CC   |M|     PT      |       sequence number         |
    // +-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
    // |                           timestamp                           |
    // +-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
    // |           synchronization source (SSRC) identifier            |
    // +=+=+=+=+=+=+=+=+=+=+=+=+=+=+=+=+=+=+=+=+=+=+=+=+=+=+=+=+=+=+=+=+
    // |            contributing source (CSRC) identifiers             |
    // |                             ....                              |
    // +-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+

    pub fn pack(&mut self, writer: &mut BytesWriter) -> Result<(), BytesWriteError> {
        let byte_1st: u8 = (self.version << 6)
            | (self.padding_flag << 5)
            | (self.extension_flag << 4)
            | (self.cc & 0x0F);
        writer.write_u8(byte_1st)?;

        let byte_2nd: u8 = (self.marker << 7) | self.payload_type;
        writer.write_u8(byte_2nd)?;

        writer.write_u16::<BigEndian>(self.seq_number)?;
        writer.write_u32::<BigEndian>(self.timestamp)?;
        writer.write_u32::<BigEndian>(self.ssrc)?;

        for csrc in &self.csrcs {
            writer.write_u32::<BigEndian>(*csrc)?;
        }

        Ok(())
    }
}
