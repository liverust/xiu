use crate::rtp::rtcp::rtcp_header::RtcpHeader;
use crate::rtp::rtcp::RTCP_SR;
use crate::rtsp_transport::ProtocolType;

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
use bytes::BytesMut;
use bytesio::bytes_reader::BytesReader;
use bytesio::bytes_writer::AsyncBytesWriter;
use bytesio::bytesio::TNetIO;
use rand::Rng;
use std::sync::Arc;
use tokio::sync::Mutex;

pub trait Track {
    fn create_packer(&mut self, writer: Arc<Mutex<Box<dyn TNetIO + Send + Sync>>>);
    fn create_unpacker(&mut self);
}
#[derive(Debug, Clone, Default, Hash, Eq, PartialEq)]
pub enum TrackType {
    #[default]
    Audio,
    Video,
    Application,
}

// A track can be a audio/video track, the A/V media data is transmitted
// over RTP, and the control data is transmitted over RTCP.
// The rtp/rtcp can be over TCP or UDP :
// 1. Over the TCP: It shares one TCP channel with the RTSP signaling data, and
// the entire session uses only one TCP connection（RTSP signaling data/audio
// RTP/audio RTCP/video RTP/video RTCP）
// 2. Over the UDP: It will establish 4 UDP channles for A/V RTP/RTCP data.
// 2.1 A RTP channel for audio media data transmitting.
// 2.2 A RTCP channel for audio control data transmitting
// 2.3 A RTP channel for video media data transmitting.
// 2.4 A RTCP channel for video control data transmitting
pub struct RtspTrack {
    track_type: TrackType,
    codec_info: RtspCodecInfo,
    pub transport: RtspTransport,
    pub uri: String,
    pub media_control: String,
    pub rtp_packer: Option<Box<dyn TPacker>>,
    //The rtp packer will be used in a separate thread when
    //received rtp data using a separate UDP channel,
    //so here we add the Arc and Mutex
    pub rtp_unpacker: Option<Arc<Mutex<Box<dyn TUnPacker>>>>,
    ssrc: u32,
    recv_ctx: RtcpContext,
    send_ctx: RtcpContext,
    init_sequence: u16,
    // The following connections are used for UDP data receiving
    rtp_io: Option<Arc<Mutex<Box<dyn TNetIO + Send + Sync>>>>,
    rtcp_io: Option<Arc<Mutex<Box<dyn TNetIO + Send + Sync>>>>,
}

impl RtspTrack {
    pub fn new(
        track_type: TrackType,
        codec_info: RtspCodecInfo,
        media_control: String,
        io: Arc<Mutex<Box<dyn TNetIO + Send + Sync>>>,
    ) -> Self {
        let ssrc: u32 = rand::thread_rng().gen();

        let mut rtsp_track = RtspTrack {
            track_type,
            codec_info,

            media_control,
            ssrc,
            transport: RtspTransport::default(),
            uri: String::default(),
            rtp_packer: None,
            rtp_unpacker: None,
            recv_ctx: RtcpContext::default(),
            send_ctx: RtcpContext::default(),
            init_sequence: 0,
            rtcp_io: None,
            rtp_io: None,
        };
        rtsp_track.create_unpacker();
        rtsp_track
    }

    // let mut cur_reader = BytesReader::new(self.reader.read_bytes(length as usize)?);

    // for (k, v) in &mut self.tracks {
    //     let mut track = v.lock().await;
    //     let rtp_identifier = track.transport.interleaved[0];
    //     let rtcp_identifier = track.transport.interleaved[1];

    //     if channel_identifier == rtp_identifier {
    //         log::debug!("onrtp: {}", cur_reader.len());

    //         track.on_rtp(&mut cur_reader);
    //         log::debug!("onrtp end: {}", cur_reader.len());
    //     } else if channel_identifier == rtcp_identifier {
    //         log::debug!("onrtcp: {}", cur_reader.len());
    //         track.on_rtcp(&mut cur_reader);
    //         log::debug!("onrtcp end: {}", cur_reader.len());
    //     }
    // }

    pub async fn rtp_receive_loop(&mut self, mut rtp_io: Box<dyn TNetIO + Send + Sync>) {
        if let Some(rtp_unpacker) = &mut self.rtp_unpacker {
            let rtp_unpacker_clone = rtp_unpacker.clone();

            tokio::spawn(async move {
                let mut reader = BytesReader::new(BytesMut::new());
                loop {
                    match rtp_io.read().await {
                        Ok(data) => {
                            reader.extend_from_slice(&data[..]);
                            if let Err(err) = rtp_unpacker_clone.lock().await.unpack(&mut reader) {
                                log::error!("unpack rtp error: {:?}", err);
                            }
                        }
                        Err(err) => {
                            log::error!("read error: {:?}", err);
                            break;
                        }
                    }
                }
            });
        }
    }

    pub async fn rtcp_receive_loop(&mut self, mut rtcp_io: Box<dyn TNetIO + Send + Sync>) {
        tokio::spawn(async move {
            let mut reader = BytesReader::new(BytesMut::new());
            loop {
                match rtcp_io.read().await {
                    Ok(data) => {
                        reader.extend_from_slice(&data[..]);
                        if let Err(err) = rtp_unpacker_clone.lock().await.unpack(&mut reader) {
                            log::error!("unpack rtp error: {:?}", err);
                        }
                    }
                    Err(err) => {
                        log::error!("read error: {:?}", err);
                        break;
                    }
                }
            }
        });
    }

    pub fn set_rtp_io(&mut self, io: Arc<Mutex<Box<dyn TNetIO + Send + Sync>>>) {
        self.rtp_io = Some(io)
    }

    pub fn set_rtcp_io(&mut self, io: Arc<Mutex<Box<dyn TNetIO + Send + Sync>>>) {
        self.rtcp_io = Some(io)
    }

    pub fn set_transport(&mut self, transport: RtspTransport) {
        self.transport = transport;
    }

    pub async fn on_rtp(&mut self, reader: &mut BytesReader) {
        if let Some(unpacker) = &mut self.rtp_unpacker {
            unpacker.lock().await.unpack(reader);
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
    fn create_unpacker(&mut self) {
        match self.codec_info.codec_id {
            RtspCodecId::H264 => {
                self.rtp_unpacker =
                    Some(Arc::new(Mutex::new(Box::new(RtpH264UnPacker::default()))));
            }
            RtspCodecId::H265 => {
                self.rtp_unpacker =
                    Some(Arc::new(Mutex::new(Box::new(RtpH265UnPacker::default()))));
            }
            RtspCodecId::AAC => {
                self.rtp_unpacker = Some(Arc::new(Mutex::new(Box::new(RtpAacUnPacker::default()))));
            }
            RtspCodecId::G711A => {}
        }
    }
    fn create_packer(&mut self, io: Arc<Mutex<Box<dyn TNetIO + Send + Sync>>>) {
        match self.codec_info.codec_id {
            RtspCodecId::H264 => {
                self.rtp_packer = Some(Box::new(RtpH264Packer::new(
                    self.codec_info.payload_type,
                    self.ssrc,
                    self.init_sequence,
                    1400,
                    io,
                )));
            }
            RtspCodecId::H265 => {
                self.rtp_packer = Some(Box::new(RtpH265Packer::new(
                    self.codec_info.payload_type,
                    self.ssrc,
                    self.init_sequence,
                    1400,
                    io,
                )));
            }
            RtspCodecId::AAC => {
                self.rtp_packer = Some(Box::new(RtpAacPacker::new(
                    self.codec_info.payload_type,
                    self.ssrc,
                    self.init_sequence,
                    1400,
                    io,
                )));
            }
            RtspCodecId::G711A => {}
        }
    }
}
