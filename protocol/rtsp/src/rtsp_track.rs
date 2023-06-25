use super::rtp::rtp_aac::RtpAacPacker;
use super::rtp::rtp_h264::RtpH264Packer;
use super::rtp::rtp_h265::RtpH265Packer;

use super::rtp::rtp_aac::RtpAacUnPacker;
use super::rtp::rtp_h264::RtpH264UnPacker;
use super::rtp::rtp_h265::RtpH265UnPacker;

use super::rtp::rtcp::rtcp_context::RtcpContext;
use super::rtp::utils::TPacker;
use super::rtp::utils::TUnPacker;
use super::rtsp_codec::RtspCodecId;
use super::rtsp_codec::RtspCodecInfo;
use super::rtsp_transport::RtspTransport;

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
    rtp_packer: Option<Box<dyn TPacker>>,
    rtp_unpacker: Option<Box<dyn TUnPacker>>,
    ssrc: u32,
    recv_ctx: RtcpContext,
    send_ctx: RtcpContext,
    init_sequence: u16,
}

impl RtspTrack {
    pub fn new(track_type: TrackType, codec_info: RtspCodecInfo, media_control: String) -> Self {
        let mut rtsp_track = RtspTrack {
            track_type,
            codec_info,
            media_control,
            ..Default::default()
        };
        rtsp_track.create_packer_unpacker();

        rtsp_track
    }

    pub fn set_transport(&mut self, transport: RtspTransport) {
        self.transport = transport;
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
