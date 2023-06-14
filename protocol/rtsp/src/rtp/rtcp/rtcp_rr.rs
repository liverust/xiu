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

impl Unmarshal<&mut BytesReader, RtcpError> for ReportBlock {
    fn unmarshal(reader: &mut BytesReader) -> Result<Self, RtcpError>
    where
        Self: Sized,
    {
        let mut report_block = ReportBlock::default();

        report_block.ssrc = reader.read_u32::<BigEndian>()?;
        report_block.fraction_lost = reader.read_u8()?;
        report_block.cumutlative_num_of_packets_lost = reader.read_u24::<BigEndian>()?;
        report_block.extended_highest_seq_number = reader.read_u32::<BigEndian>()?;
        report_block.jitter = reader.read_u32::<BigEndian>()?;
        report_block.lsr = reader.read_u32::<BigEndian>()?;
        report_block.dlsr = reader.read_u32::<BigEndian>()?;

        Ok(report_block)
    }
}

impl Marshal<RtcpError> for ReportBlock {
    fn marshal(&self) -> Result<BytesMut, RtcpError> {
        let mut writer = BytesWriter::default();

        writer.write_u32::<BigEndian>(self.ssrc)?;
        writer.write_u8(self.fraction_lost)?;
        writer.write_u24::<BigEndian>(self.cumutlative_num_of_packets_lost)?;
        writer.write_u32::<BigEndian>(self.extended_highest_seq_number)?;
        writer.write_u32::<BigEndian>(self.jitter)?;
        writer.write_u32::<BigEndian>(self.lsr)?;
        writer.write_u32::<BigEndian>(self.dlsr)?;

        Ok(writer.extract_current_bytes())
    }
}

#[derive(Debug, Clone, Default)]
pub struct ReceiverReport {
    header: RtcpHeader,
    ssrc: u32,
    report_blocks: Vec<ReportBlock>,
}

impl Unmarshal<BytesMut, RtcpError> for ReceiverReport {
    fn unmarshal(data: BytesMut) -> Result<Self, RtcpError>
    where
        Self: Sized,
    {
        let mut reader = BytesReader::new(data);

        let mut receiver_report = ReceiverReport::default();
        receiver_report.header = RtcpHeader::unmarshal(&mut reader)?;
        receiver_report.ssrc = reader.read_u32::<BigEndian>()?;

        for _ in 0..receiver_report.header.report_count {
            let report_block = ReportBlock::unmarshal(&mut reader)?;
            receiver_report.report_blocks.push(report_block);
        }

        Ok(receiver_report)
    }
}

impl Marshal<RtcpError> for ReceiverReport {
    fn marshal(&self) -> Result<BytesMut, RtcpError> {
        let mut writer = BytesWriter::default();

        let header_bytesmut = self.header.marshal()?;
        writer.write(&header_bytesmut[..])?;

        writer.write_u32::<BigEndian>(self.ssrc)?;
        for report_block in &self.report_blocks {
            let data = report_block.marshal()?;
            writer.write(&data[..])?;
        }

        Ok(writer.extract_current_bytes())
    }
}
