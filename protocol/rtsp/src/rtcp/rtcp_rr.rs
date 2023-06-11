use super::rtcp_header::RtcpHeader;
use byteorder::BigEndian;
use bytes::{BufMut, BytesMut};
use bytesio::bytes_errors::BytesReadError;
use bytesio::bytes_errors::BytesWriteError;
use bytesio::bytes_reader::BytesReader;
use bytesio::bytes_writer::BytesWriter;

#[derive(Debug, Clone, Default)]
struct ReportBlock {
    ssrc: u32,
    fraction_lost: u8,
    cumutlative_num_of_packets_lost: u32,
    extended_highest_seq_number: u32,
    jitter: u32,
    lsr: u32,
    dlsr: u32,
}

impl ReportBlock {
    pub fn unpack(&mut self, reader: &mut BytesReader) -> Result<(), BytesReadError> {
        self.ssrc = reader.read_u32::<BigEndian>()?;
        self.fraction_lost = reader.read_u8()?;
        self.cumutlative_num_of_packets_lost = reader.read_u24::<BigEndian>()?;
        self.extended_highest_seq_number = reader.read_u32::<BigEndian>()?;
        self.jitter = reader.read_u32::<BigEndian>()?;
        self.lsr = reader.read_u32::<BigEndian>()?;
        self.dlsr = reader.read_u32::<BigEndian>()?;

        Ok(())
    }

    pub fn pack(&self, writer: &mut BytesWriter) -> Result<(), BytesWriteError> {
        writer.write_u32::<BigEndian>(self.ssrc)?;
        writer.write_u8(self.fraction_lost)?;
        writer.write_u24::<BigEndian>(self.cumutlative_num_of_packets_lost)?;
        writer.write_u32::<BigEndian>(self.extended_highest_seq_number)?;
        writer.write_u32::<BigEndian>(self.jitter)?;
        writer.write_u32::<BigEndian>(self.lsr)?;
        writer.write_u32::<BigEndian>(self.dlsr)?;
        Ok(())
    }
}

pub struct ReceiverReport {
    header: RtcpHeader,
    ssrc: u32,
    report_blocks: Vec<ReportBlock>,
}

impl ReceiverReport {
    pub fn unpack(&mut self, data: BytesMut) -> Result<(), BytesReadError> {
        let mut reader = BytesReader::new(data);

        self.header.unpack(&mut reader)?;
        self.ssrc = reader.read_u32::<BigEndian>()?;

        for _ in 0..self.header.report_count {
            let mut report_block = ReportBlock::default();
            report_block.unpack(&mut reader)?;
            self.report_blocks.push(report_block);
        }
        Ok(())
    }

    pub fn pack(&mut self, writer: &mut BytesWriter) -> Result<(), BytesWriteError> {
        self.header.pack(writer)?;

        writer.write_u32::<BigEndian>(self.ssrc)?;

        for report_block in &self.report_blocks {
            report_block.pack(writer)?;
        }
        Ok(())
    }
}
