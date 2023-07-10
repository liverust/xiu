use crate::rtp::rtcp::rtcp_header::RtcpHeader;
use crate::rtp::rtcp::RTCP_SR;

use super::rtp::rtp_aac::RtpAacPacker;
use super::rtp::rtp_h264::RtpH264Packer;
use super::rtp::rtp_h265::RtpH265Packer;

use super::rtp::rtp_aac::RtpAacUnPacker;
use super::rtp::rtp_h264::RtpH264UnPacker;
use super::rtp::rtp_h265::RtpH265UnPacker;

use super::rtp::rtcp::rtcp_context::RtcpContext;
use super::rtp::rtcp::rtcp_sr::RtcpSenderReport;
use super::rtp::utils::TPacker;
use super::rtp::utils::TUnPacker;
use super::rtsp_codec::RtspCodecId;
use super::rtsp_codec::RtspCodecInfo;
use super::rtsp_transport::RtspTransport;
use crate::rtp::utils::Marshal;
use crate::rtp::utils::Unmarshal;
use bytesio::bytes_reader::BytesReader;
use rand::Rng;

trait Track {
    fn create_packer_unpacker(&mut self);
}
#[derive(Debug, Clone, Default, Hash, Eq, PartialEq)]
pub enum TrackType {
    #[default]
    Audio,
    Video,
    Application,
}

#[derive(Default)]
pub struct RtspTrack {
    track_type: TrackType,
    codec_info: RtspCodecInfo,
    pub transport: RtspTransport,
    pub uri: String,
    pub media_control: String,
    pub rtp_packer: Option<Box<dyn TPacker>>,
    pub rtp_unpacker: Option<Box<dyn TUnPacker>>,
    ssrc: u32,
    recv_ctx: RtcpContext,
    send_ctx: RtcpContext,
    init_sequence: u16,
}

impl RtspTrack {
    pub fn new(track_type: TrackType, codec_info: RtspCodecInfo, media_control: String) -> Self {
        let ssrc: u32 = rand::thread_rng().gen();

        let mut rtsp_track = RtspTrack {
            track_type,
            codec_info,
            media_control,
            ssrc,
            ..Default::default()
        };
        rtsp_track.create_packer_unpacker();
        rtsp_track
    }

    pub fn set_transport(&mut self, transport: RtspTransport) {
        self.transport = transport;
    }

    pub fn on_rtp(&mut self, reader: &mut BytesReader) {
        if let Some(unpacker) = &mut self.rtp_unpacker {
            unpacker.unpack(reader);
        }
    }

    pub fn on_rtcp(&mut self, reader: &mut BytesReader) {
        let mut reader_clone = BytesReader::new(reader.get_remaining_bytes());
        if let Ok(rtcp_header) = RtcpHeader::unmarshal(&mut reader_clone) {
            match rtcp_header.payload_type {
                RTCP_SR => {
                    if let Ok(sr) = RtcpSenderReport::unmarshal(reader) {
                        self.recv_ctx.received_sr(&sr);
                        self.send_rtcp_receier_report();
                    }
                }
                _ => {}
            }
        }
    }

    pub fn send_rtcp_receier_report(&mut self) {
        let rr = self.recv_ctx.generate_rr();
        let data = rr.marshal();
    }
}

impl Track for RtspTrack {
    fn create_packer_unpacker(&mut self) {
        match self.codec_info.codec_id {
            RtspCodecId::H264 => {
                self.rtp_packer = Some(Box::new(RtpH264Packer::new(
                    self.codec_info.payload_type,
                    self.ssrc,
                    self.init_sequence,
                    1400,
                )));
                self.rtp_unpacker = Some(Box::new(RtpH264UnPacker::default()));
            }
            RtspCodecId::H265 => {
                self.rtp_packer = Some(Box::new(RtpH265Packer::new(
                    self.codec_info.payload_type,
                    self.ssrc,
                    self.init_sequence,
                    1400,
                )));
                self.rtp_unpacker = Some(Box::new(RtpH265UnPacker::default()));
            }
            RtspCodecId::AAC => {
                self.rtp_packer = Some(Box::new(RtpAacPacker::new(
                    self.codec_info.payload_type,
                    self.ssrc,
                    self.init_sequence,
                    1400,
                )));
                self.rtp_unpacker = Some(Box::new(RtpAacUnPacker::default()));
            }
            RtspCodecId::G711A => {}
        }
    }
}
