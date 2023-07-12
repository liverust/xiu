pub mod define;
pub mod errors;
use crate::global_trait::Marshal;
use crate::http::parser::RtspResponse;
use crate::rtsp_range::RtspRange;

use super::rtsp_codec;
use crate::global_trait::Unmarshal;

use crate::rtsp_codec::RtspCodecInfo;
use crate::rtsp_track::RtspTrack;
use crate::rtsp_track::TrackType;
use crate::rtsp_transport::ProtocolType;
use crate::rtsp_transport::RtspTransport;
use crate::rtsp_utils;
use byteorder::BigEndian;
use bytes::BytesMut;
use bytesio::bytes_reader::BytesReader;
use bytesio::bytes_writer::AsyncBytesWriter;

use bytesio::bytesio::UdpIO;
use errors::SessionError;
use errors::SessionErrorValue;
use http::StatusCode;
use streamhub::define::Information;

use super::http::parser::RtspRequest;
use super::rtp::errors::UnPackerError;
use super::rtp::utils::Unmarshal as RtpUnmarshal;
use super::rtp::RtpPacket;
use super::rtsp_track::Track;
use super::sdp::Sdp;

use async_trait::async_trait;
use bytesio::bytesio::TNetIO;
use bytesio::bytesio::TcpIO;
use define::rtsp_method_name;

use std::collections::HashMap;
use std::str::FromStr;
use std::sync::Arc;
use tokio::sync::mpsc;
use uuid::Uuid;

use streamhub::{
    define::{
        FrameData, FrameDataSender, InformationSender, NotifyInfo, PublishType, PublisherInfo,
        StreamHubEvent, StreamHubEventSender, SubscribeType, SubscriberInfo, TStreamHandler,
    },
    errors::ChannelError,
    statistics::StreamStatistics,
    stream::StreamIdentifier,
};
use tokio::net::TcpStream;
use tokio::sync::Mutex;

pub struct RtspServerSession {
    io: Arc<Mutex<Box<dyn TNetIO + Send + Sync>>>,
    reader: BytesReader,
    writer: AsyncBytesWriter,

    transport: RtspTransport,
    tracks: HashMap<TrackType, RtspTrack>,
    sdp: Sdp,
    pub session_id: Option<String>,

    stream_handler: Arc<RtspStreamHandler>,
    event_producer: StreamHubEventSender,
}

pub struct InterleavedBinaryData {
    channel_identifier: u8,
    length: u16,
}

impl InterleavedBinaryData {
    // 10.12 Embedded (Interleaved) Binary Data
    // Stream data such as RTP packets is encapsulated by an ASCII dollar
    // sign (24 hexadecimal), followed by a one-byte channel identifier,
    // followed by the length of the encapsulated binary data as a binary,
    // two-byte integer in network byte order
    fn new(reader: &mut BytesReader) -> Result<Option<Self>, SessionError> {
        let is_dollar_sign = reader.advance_u8()? == 0x24;
        log::debug!("dollar sign: {}", is_dollar_sign);
        if is_dollar_sign {
            reader.read_u8()?;
            let channel_identifier = reader.read_u8()?;
            log::debug!("channel_identifier: {}", channel_identifier);
            let length = reader.read_u16::<BigEndian>()?;
            log::debug!("length: {}", length);
            return Ok(Some(InterleavedBinaryData {
                channel_identifier,
                length,
            }));
        }
        Ok(None)
    }
}

impl RtspServerSession {
    pub fn new(stream: TcpStream, event_producer: StreamHubEventSender) -> Self {
        let remote_addr = if let Ok(addr) = stream.peer_addr() {
            log::info!("server session: {}", addr.to_string());
            Some(addr)
        } else {
            None
        };

        let net_io: Box<dyn TNetIO + Send + Sync> = Box::new(TcpIO::new(stream));
        let io = Arc::new(Mutex::new(net_io));

        Self {
            io: io.clone(),
            reader: BytesReader::new(BytesMut::default()),
            writer: AsyncBytesWriter::new(io),
            transport: RtspTransport::default(),
            tracks: HashMap::new(),
            sdp: Sdp::default(),
            session_id: None,
            event_producer,
            stream_handler: Arc::new(RtspStreamHandler::new()),
        }
    }

    pub async fn run(&mut self) -> Result<(), SessionError> {
        loop {
            while self.reader.len() < 4 {
                let data = self.io.lock().await.read().await?;
                self.reader.extend_from_slice(&data[..]);
            }

            // log::info!("read length========= :{}", self.reader.len());

            if let Ok(data) = InterleavedBinaryData::new(&mut self.reader) {
                match data {
                    Some(a) => {
                        if self.reader.len() < a.length as usize {
                            let data = self.io.lock().await.read().await?;
                            self.reader.extend_from_slice(&data[..]);
                        }
                        self.on_rtp_over_rtsp_message(a)?;
                    }
                    None => {
                        //log::info!("read length 2========= :{}", self.reader.len());
                        self.on_rtsp_message().await?;
                    }
                }
            }

            //log::info!("left bytes: {}", self.reader.len());
        }
    }

    fn on_rtp_over_rtsp_message(
        &mut self,
        interleaved_binary_data: InterleavedBinaryData,
    ) -> Result<(), SessionError> {
        let mut cur_reader = BytesReader::new(
            self.reader
                .read_bytes(interleaved_binary_data.length as usize)?,
        );

        for (k, v) in &mut self.tracks {
            let rtp_identifier = v.transport.interleaved[0];
            let rtcp_identifier = v.transport.interleaved[1];

            if interleaved_binary_data.channel_identifier == rtp_identifier {
                log::debug!("onrtp: {}", cur_reader.len());

                v.on_rtp(&mut cur_reader);
                log::debug!("onrtp end: {}", cur_reader.len());
            } else if interleaved_binary_data.channel_identifier == rtcp_identifier {
                log::debug!("onrtcp: {}", cur_reader.len());
                v.on_rtcp(&mut cur_reader);
                log::debug!("onrtcp end: {}", cur_reader.len());
            }
        }
        Ok(())
    }

    //publish stream: OPTIONS->ANNOUNCE->SETUP->RECORD->TEARDOWN
    //subscribe stream: OPTIONS->DESCRIBE->SETUP->PLAY
    async fn on_rtsp_message(&mut self) -> Result<(), SessionError> {
        let data = self.reader.extract_remaining_bytes();

        if let Some(rtsp_request) = RtspRequest::unmarshal(std::str::from_utf8(&data)?) {
            match rtsp_request.method.as_str() {
                rtsp_method_name::OPTIONS => {
                    self.handle_options(&rtsp_request).await?;
                }
                rtsp_method_name::DESCRIBE => {
                    self.handle_describe(&rtsp_request).await?;
                }
                rtsp_method_name::ANNOUNCE => {
                    self.handle_announce(&rtsp_request).await?;
                }
                rtsp_method_name::SETUP => {
                    self.handle_setup(&rtsp_request).await?;
                }
                rtsp_method_name::PLAY => {
                    self.handle_play(&rtsp_request).await?;
                }
                rtsp_method_name::RECORD => {
                    self.handle_record(&rtsp_request).await?;
                }
                rtsp_method_name::PAUSE => {}
                rtsp_method_name::TEARDOWN => {}
                rtsp_method_name::GET_PARAMETER => {}
                rtsp_method_name::SET_PARAMETER => {}
                rtsp_method_name::REDIRECT => {}

                _ => {}
            }
        }

        Ok(())
    }

    async fn handle_options(&mut self, rtsp_request: &RtspRequest) -> Result<(), SessionError> {
        let status_code = http::StatusCode::OK;
        let mut response = Self::gen_response(status_code, &rtsp_request);
        let public_str = rtsp_method_name::ARRAY.join(",");
        response.headers.insert("Public".to_string(), public_str);
        self.send_response(&response).await?;

        Ok(())
    }

    async fn handle_describe(&mut self, rtsp_request: &RtspRequest) -> Result<(), SessionError> {
        let status_code = http::StatusCode::OK;

        // The sender is used for sending sdp information from the server session to client session
        // receiver is used to receive the sdp information
        let (sender, mut receiver) = mpsc::unbounded_channel();

        let request_event = StreamHubEvent::Request {
            identifier: StreamIdentifier::Rtsp {
                stream_path: rtsp_request.path.clone(),
            },
            sender,
        };

        if self.event_producer.send(request_event).is_err() {
            return Err(SessionError {
                value: SessionErrorValue::StreamHubEventSendErr,
            });
        }

        if let Some(Information::Sdp { data }) = receiver.recv().await {
            if let Some(sdp) = Sdp::unmarshal(&data) {
                self.sdp = sdp;
                //it can new tracks when get the sdp information;
                self.new_tracks()?;
            }
        }

        let mut response = Self::gen_response(status_code, &rtsp_request);
        let sdp = self.sdp.marshal();
        log::info!("sdp: {}", sdp);
        response.body = Some(sdp);
        response
            .headers
            .insert("Content-Type".to_string(), "application/sdp".to_string());
        self.send_response(&response).await?;

        Ok(())
    }

    async fn handle_announce(&mut self, rtsp_request: &RtspRequest) -> Result<(), SessionError> {
        if let Some(request_body) = &rtsp_request.body {
            if let Some(sdp) = Sdp::unmarshal(&request_body) {
                self.sdp = sdp.clone();
                self.stream_handler.set_sdp(sdp).await;
            }
        }

        //new tracks for publish session
        self.new_tracks()?;

        // The sender is used for sending audio/video frame data to stream hub
        // receiver is used to passing to stream hub and receive the a/v frame data
        let (sender, receiver) = mpsc::unbounded_channel();
        for (_, track) in &mut self.tracks {
            if let Some(unpacker) = &mut track.rtp_unpacker {
                let sender_out = sender.clone();
                unpacker.on_frame_handler(Box::new(
                    move |msg: FrameData| -> Result<(), UnPackerError> {
                        if let Err(err) = sender_out.send(msg) {
                            log::error!("send frame error: {}", err);
                        }
                        Ok(())
                    },
                ));
            }
        }

        let publish_event = StreamHubEvent::Publish {
            identifier: StreamIdentifier::Rtsp {
                stream_path: rtsp_request.path.clone(),
            },
            receiver,
            info: self.get_publisher_info(),
            stream_handler: self.stream_handler.clone(),
        };

        if self.event_producer.send(publish_event).is_err() {
            return Err(SessionError {
                value: SessionErrorValue::StreamHubEventSendErr,
            });
        }

        let status_code = http::StatusCode::OK;
        let response = Self::gen_response(status_code, &rtsp_request);
        self.send_response(&response).await?;

        Ok(())
    }

    async fn handle_setup(&mut self, rtsp_request: &RtspRequest) -> Result<(), SessionError> {
        let status_code = http::StatusCode::OK;
        let mut response = Self::gen_response(status_code, &rtsp_request);

        for (_, v) in &mut self.tracks {
            if !rtsp_request.url.contains(&v.media_control) {
                continue;
            }

            if let Some(transport_data) = rtsp_request.get_header(&"Transport".to_string()) {
                if self.session_id.is_none() {
                    self.session_id = Some(rtsp_utils::gen_random_string(10));
                }

                let transport = RtspTransport::unmarshal(transport_data);

                if let Some(trans) = transport {
                    log::info!("get transport");
                    match trans.protocol_type {
                        ProtocolType::TCP => {
                            log::info!("get transport TCP");
                            v.create_packer(self.io.clone());
                        }
                        ProtocolType::UDP => {
                            log::info!("get transport UDP");
                            let address = rtsp_request.address.clone();
                            let rtp_port = trans.client_port[0] as u16;
                            let rtcp_port = trans.client_port[1];

                            if let Some(udp_io) = UdpIO::new(address, rtp_port).await {
                                let box_udp_io: Box<dyn TNetIO + Send + Sync> = Box::new(udp_io);
                                let io = Arc::new(Mutex::new(box_udp_io));
                                v.create_packer(io);
                            }

                            // let net_io: Box<dyn TNetIO + Send + Sync> = Box::new(TcpIO::new(se));
                            // let io = Arc::new(Mutex::new(net_io));
                        }
                    }

                    v.set_transport(trans);

                    // v.create_packer_unpacker();
                }

                response
                    .headers
                    .insert("Transport".to_string(), transport_data.clone());
                response
                    .headers
                    .insert("Session".to_string(), self.session_id.clone().unwrap());
            }
            break;
        }

        self.send_response(&response).await?;

        Ok(())
    }

    async fn handle_play(&mut self, rtsp_request: &RtspRequest) -> Result<(), SessionError> {
        for (t, track) in &mut self.tracks {
            if let Some(packer) = &mut track.rtp_packer {
                let channel_identifer = track.transport.interleaved[0];

                packer.on_packet_handler(Box::new(
                    move |io: Arc<Mutex<Box<dyn TNetIO + Send + Sync>>>, msg: BytesMut| {
                        Box::pin(async move {
                            let mut bytes_writer = AsyncBytesWriter::new(io);

                            bytes_writer.write_u8(0x24)?;
                            bytes_writer.write_u8(channel_identifer)?;
                            bytes_writer.write_u16::<BigEndian>(msg.len() as u16)?;
                            bytes_writer.write(&msg)?;
                            bytes_writer.flush().await?;

                            Ok(())
                        })
                    },
                ));
            }
        }

        let status_code = http::StatusCode::OK;
        let response = Self::gen_response(status_code, &rtsp_request);

        self.send_response(&response).await?;

        // The sender is used for sending audio/video frame data to stream hub
        // receiver is used to passing to stream hub and receive the a/v frame data
        let (sender, mut receiver) = mpsc::unbounded_channel();

        let publish_event = StreamHubEvent::Subscribe {
            identifier: StreamIdentifier::Rtsp {
                stream_path: rtsp_request.path.clone(),
            },
            sender,
            info: self.get_subscriber_info(),
        };

        if self.event_producer.send(publish_event).is_err() {
            return Err(SessionError {
                value: SessionErrorValue::StreamHubEventSendErr,
            });
        }

        loop {
            if let Some(frame_data) = receiver.recv().await {
                match frame_data {
                    FrameData::Audio {
                        timestamp,
                        mut data,
                    } => {
                        if let Some(audio_track) = self.tracks.get_mut(&TrackType::Audio) {
                            if let Some(packer) = &mut audio_track.rtp_packer {
                                packer.pack(&mut data, timestamp).await?;
                            }
                        }
                    }
                    FrameData::Video {
                        timestamp,
                        mut data,
                    } => {
                        if let Some(video_track) = self.tracks.get_mut(&TrackType::Video) {
                            if let Some(packer) = &mut video_track.rtp_packer {
                                packer.pack(&mut data, timestamp).await?;
                            }
                        }
                    }
                    FrameData::MetaData { timestamp, data } => {}
                }
            }
        }
        Ok(())
    }

    async fn handle_record(&mut self, rtsp_request: &RtspRequest) -> Result<(), SessionError> {
        if let Some(range_str) = rtsp_request.headers.get(&String::from("Range")) {
            if let Some(range) = RtspRange::unmarshal(&range_str) {
                let status_code = http::StatusCode::OK;
                let mut response = Self::gen_response(status_code, &rtsp_request);
                response
                    .headers
                    .insert(String::from("Range"), range.marshal());
                response
                    .headers
                    .insert("Session".to_string(), self.session_id.clone().unwrap());

                self.send_response(&response).await?;
            }
        }

        Ok(())
    }

    fn new_tracks(&mut self) -> Result<(), SessionError> {
        let writer = self.io.clone();

        for media in &self.sdp.medias {
            let media_control = if let Some(media_control_val) = media.attributes.get("control") {
                media_control_val.clone()
            } else {
                String::from("")
            };

            let media_name = &media.media_type;
            log::info!("media_name: {}", media_name);
            match media_name.as_str() {
                "audio" => {
                    let codec_id = rtsp_codec::RTSP_CODEC_NAME_2_ID
                        .get(&media.rtpmap.encoding_name.to_lowercase().as_str())
                        .unwrap()
                        .clone();
                    let codec_info = RtspCodecInfo {
                        codec_id,
                        payload_type: media.rtpmap.payload_type as u8,
                        sample_rate: media.rtpmap.clock_rate,
                        channel_count: media.rtpmap.encoding_param.parse().unwrap(),
                    };

                    log::info!("audio codec info: {:?}", codec_info);

                    let track = RtspTrack::new(
                        TrackType::Audio,
                        codec_info,
                        media_control,
                        self.io.clone(),
                    );
                    self.tracks.insert(TrackType::Audio, track);
                }
                "video" => {
                    let codec_id = rtsp_codec::RTSP_CODEC_NAME_2_ID
                        .get(&media.rtpmap.encoding_name.to_lowercase().as_str())
                        .unwrap()
                        .clone();
                    let codec_info = RtspCodecInfo {
                        codec_id,
                        payload_type: media.rtpmap.payload_type as u8,
                        sample_rate: media.rtpmap.clock_rate,
                        ..Default::default()
                    };
                    let track = RtspTrack::new(
                        TrackType::Video,
                        codec_info,
                        media_control,
                        self.io.clone(),
                    );
                    self.tracks.insert(TrackType::Video, track);
                }
                _ => {}
            }
        }
        Ok(())
    }

    fn gen_response(status_code: StatusCode, rtsp_request: &RtspRequest) -> RtspResponse {
        let reason_phrase = if let Some(reason) = status_code.canonical_reason() {
            reason.to_string()
        } else {
            "".to_string()
        };

        let mut response = RtspResponse {
            version: "RTSP/1.0".to_string(),
            status_code: status_code.as_u16(),
            reason_phrase,
            ..Default::default()
        };

        if let Some(cseq) = rtsp_request.headers.get("CSeq") {
            response
                .headers
                .insert("CSeq".to_string(), cseq.to_string());
        }

        response
    }

    fn get_subscriber_info(&mut self) -> SubscriberInfo {
        let id = if let Ok(uuid) = Uuid::from_str(&self.session_id.clone().unwrap()) {
            uuid
        } else {
            Uuid::new_v4()
        };

        SubscriberInfo {
            id,
            sub_type: SubscribeType::PlayerRtsp,
            notify_info: NotifyInfo {
                request_url: String::from(""),
                remote_addr: String::from(""),
            },
        }
    }

    fn get_publisher_info(&mut self) -> PublisherInfo {
        let id = if let Some(session_id) = &self.session_id {
            if let Ok(uuid) = Uuid::from_str(&session_id) {
                uuid
            } else {
                Uuid::new_v4()
            }
        } else {
            Uuid::new_v4()
        };

        PublisherInfo {
            id,
            pub_type: PublishType::PushRtsp,
            notify_info: NotifyInfo {
                request_url: String::from(""),
                remote_addr: String::from(""),
            },
        }
    }

    async fn send_response(&mut self, response: &RtspResponse) -> Result<(), SessionError> {
        self.writer.write(response.marshal().as_bytes())?;
        self.writer.flush().await?;

        Ok(())
        //response.
    }
}

pub struct RtspStreamHandler {
    sdp: Mutex<Sdp>,
}

impl RtspStreamHandler {
    pub fn new() -> Self {
        Self {
            sdp: Mutex::new(Sdp::default()),
        }
    }
    pub async fn set_sdp(&self, sdp: Sdp) {
        *self.sdp.lock().await = sdp;
    }
}

#[async_trait]
impl TStreamHandler for RtspStreamHandler {
    async fn send_cache_data(
        &self,
        sender: FrameDataSender,
        sub_type: SubscribeType,
    ) -> Result<(), ChannelError> {
        Ok(())
    }
    async fn get_statistic_data(&self) -> Option<StreamStatistics> {
        None
    }

    async fn send_information(&self, sender: InformationSender) {
        if let Err(err) = sender.send(Information::Sdp {
            data: self.sdp.lock().await.marshal(),
        }) {
            log::error!("send_information of rtsp error: {}", err);
        }
    }
}
