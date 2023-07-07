pub mod define;
pub mod errors;
use crate::global_trait::Marshal;
use crate::http::parser::RtspResponse;

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
use chrono::format::InternalFixed;
use errors::SessionError;
use errors::SessionErrorValue;
use http::StatusCode;

use super::http::parser::RtspRequest;
use super::rtp::errors::UnPackerError;
use super::sdp::Sdp;
use super::session::define::SessionType;
use async_trait::async_trait;
use bytesio::bytesio::BytesIO;
use define::rtsp_method_name;
use httparse::Request;
use httparse::Response;
use indexmap::indexmap;
use indexmap::IndexMap;
use std::collections::HashMap;
use std::str::FromStr;
use std::sync::Arc;
use tokio::sync::{mpsc, oneshot};
use uuid::Uuid;

use streamhub::{
    define::{
        FrameData, FrameDataReceiver, FrameDataSender, NotifyInfo, PubSubInfo, PublishType,
        PublisherInfo, StreamHubEvent, StreamHubEventSender, SubscribeType, SubscriberInfo,
        TStreamHandler,
    },
    errors::{ChannelError, ChannelErrorValue},
    statistics::StreamStatistics,
    stream::StreamIdentifier,
};
use tokio::net::TcpStream;
use tokio::sync::Mutex;

pub struct RtspServerSession {
    io: Arc<Mutex<BytesIO>>,
    reader: BytesReader,
    writer: AsyncBytesWriter,

    transport: RtspTransport,
    tracks: HashMap<TrackType, RtspTrack>,
    sdp: Sdp,
    pub session_id: String,
    stream_handler: Arc<RtspStreamHandler>,

    event_producer: StreamHubEventSender,
    data_sender: FrameDataSender,
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
        if is_dollar_sign {
            reader.read_u8()?;
            let channel_identifier = reader.read_u8()?;
            let length = reader.read_u16::<BigEndian>()?;
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

        let io = Arc::new(Mutex::new(BytesIO::new(stream)));

        let (init_sender, init_receiver) = mpsc::unbounded_channel();

        Self {
            io: io.clone(),
            reader: BytesReader::new(BytesMut::default()),
            writer: AsyncBytesWriter::new(io),
            transport: RtspTransport::default(),
            tracks: HashMap::new(),
            sdp: Sdp::default(),
            session_id: String::default(),
            event_producer,
            stream_handler: Arc::new(RtspStreamHandler::new()),
            data_sender: init_sender,
        }
    }

    pub async fn run(&mut self) -> Result<(), SessionError> {
        loop {
            let data = self.io.lock().await.read().await?;
            self.reader.extend_from_slice(&data[..]);

            if let Ok(data) = InterleavedBinaryData::new(&mut self.reader) {
                match data {
                    Some(a) => {
                        self.on_rtp_over_rtsp_message(a);
                    }
                    None => {
                        self.on_rtsp_message().await?;
                    }
                }
            }
        }
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

    async fn on_rtsp_message(&mut self) -> Result<(), SessionError> {
        let data = self.reader.extract_remaining_bytes();

        if let Some(rtsp_request) = RtspRequest::unmarshal(std::str::from_utf8(&data)?) {
            match rtsp_request.method.as_str() {
                rtsp_method_name::OPTIONS => {
                    let status_code = http::StatusCode::OK;
                    let mut response = Self::gen_response(status_code, &rtsp_request);
                    let public_str = rtsp_method_name::ARRAY.join(",");
                    response.headers.insert("Public".to_string(), public_str);

                    self.send_response(&response)?;
                }
                rtsp_method_name::DESCRIBE => {
                    let status_code = http::StatusCode::OK;
                    let mut response = Self::gen_response(status_code, &rtsp_request);
                    response.body = Some(self.sdp.marshal());
                    response
                        .headers
                        .insert("Content-Type".to_string(), "application/sdp".to_string());

                    self.send_response(&response)?;
                }
                rtsp_method_name::ANNOUNCE => {
                    if let Some(request_body) = rtsp_request.body {
                        if let Some(sdp) = Sdp::unmarshal(&request_body) {
                            self.sdp = sdp;
                        }
                    }

                    for media in &self.sdp.medias {
                        let media_control =
                            if let Some(media_control_val) = media.attributes.get("control") {
                                media_control_val.clone()
                            } else {
                                String::from("")
                            };

                        let media_name = &media.media_type;
                        match media_name.as_str() {
                            "audio" => {
                                let codec_id = rtsp_codec::RTSP_CODEC_NAME_2_ID
                                    .get(&media_name.as_str())
                                    .unwrap()
                                    .clone();
                                let codec_info = RtspCodecInfo {
                                    codec_id,
                                    payload_type: media.rtpmap.payload_type as u8,
                                    sample_rate: media.rtpmap.clock_rate,
                                    channel_count: media.rtpmap.encoding_param.parse().unwrap(),
                                };

                                let track =
                                    RtspTrack::new(TrackType::Audio, codec_info, media_control);

                                self.tracks.insert(TrackType::Audio, track);
                            }
                            "video" => {
                                let codec_id = rtsp_codec::RTSP_CODEC_NAME_2_ID
                                    .get(&media_name.as_str())
                                    .unwrap()
                                    .clone();
                                let codec_info = RtspCodecInfo {
                                    codec_id,
                                    payload_type: media.rtpmap.payload_type as u8,
                                    sample_rate: media.rtpmap.clock_rate,
                                    ..Default::default()
                                };
                                let track =
                                    RtspTrack::new(TrackType::Video, codec_info, media_control);
                                self.tracks.insert(TrackType::Video, track);
                            }
                            _ => {}
                        }
                    }

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
                            stream_path: rtsp_request.path,
                        },
                        receiver,
                        info: self.get_publisher_info(),
                        stream_handler: self.stream_handler.clone(),
                    };

                    let rv = self.event_producer.send(publish_event);
                    if rv.is_err() {
                        return Err(SessionError {
                            value: SessionErrorValue::StreamHubEventSendErr,
                        });
                    }

                    self.data_sender = sender;
                }
                rtsp_method_name::SETUP => {
                    let status_code = http::StatusCode::OK;
                    let mut response = Self::gen_response(status_code, &rtsp_request);

                    for (_, v) in &mut self.tracks {
                        if !rtsp_request.url.contains(&v.media_control) {
                            continue;
                        }

                        if let Some(transport_data) =
                            rtsp_request.get_header(&"Transport".to_string())
                        {
                            self.session_id = rtsp_utils::gen_random_string(10);

                            let transport = RtspTransport::unmarshal(transport_data);
                            if let Some(trans) = transport {
                                if trans.protocol_type == ProtocolType::TCP {}
                                v.set_transport(trans);
                            }

                            response
                                .headers
                                .insert("Transport".to_string(), transport_data.clone());
                            response
                                .headers
                                .insert("Session".to_string(), self.session_id.clone());
                        }
                        break;
                    }

                    self.send_response(&response)?;
                }
                rtsp_method_name::PLAY => {}
                rtsp_method_name::PAUSE => {}
                rtsp_method_name::TEARDOWN => {}
                rtsp_method_name::GET_PARAMETER => {}
                rtsp_method_name::SET_PARAMETER => {}
                rtsp_method_name::REDIRECT => {}
                rtsp_method_name::RECORD => {}
                _ => {}
            }
        }

        Ok(())
    }

    fn get_publisher_info(&mut self) -> PublisherInfo {
        let id = if let Ok(uuid) = Uuid::from_str(&self.session_id) {
            uuid
        } else {
            Uuid::from_str("unknown").unwrap()
        };

        PublisherInfo {
            id,
            sub_type: PublishType::PushRtsp,
            notify_info: NotifyInfo {
                request_url: String::from(""),
                remote_addr: String::from(""),
            },
        }
    }

    fn on_rtp_over_rtsp_message(&mut self, interleaved_binary_data: InterleavedBinaryData) {
        for (k, v) in &mut self.tracks {
            let rtp_identifier = v.transport.interleaved[0];
            let rtcp_identifier = v.transport.interleaved[1];

            match interleaved_binary_data.channel_identifier {
                rtp_identifier => v.on_rtp(&mut self.reader),
                rtcp_identifier => v.on_rtcp(&mut self.reader),
                _ => {}
            }
            //if v.transport.interleaved[0] == interleaved_binary_data.channel_identifier {}
        }
    }
    fn send_response(&mut self, response: &RtspResponse) -> Result<(), SessionError> {
        self.writer.write(response.marshal().as_bytes())?;

        Ok(())
        //response.
    }
}

pub struct RtspStreamHandler {}

impl RtspStreamHandler {
    pub fn new() -> Self {
        Self {}
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
}
