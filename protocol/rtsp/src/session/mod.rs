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
use bytesio::bytes_reader::AsyncBytesReader;
use bytesio::bytes_writer::AsyncBytesWriter;
use errors::SessionError;
use http::StatusCode;

use super::http::parser::RtspRequest;
use super::sdp::Sdp;
use define::rtsp_method_name;
use httparse::Request;
use httparse::Response;
use indexmap::indexmap;
use indexmap::IndexMap;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::Mutex;

pub struct RtspServerSession {
    reader: AsyncBytesReader,
    writer: AsyncBytesWriter,
    bytesio_data: BytesMut,
    transport: RtspTransport,
    tracks: HashMap<TrackType, RtspTrack>,
    sdp: Sdp,
    pub session_id: String,
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
    async fn new(reader: &mut AsyncBytesReader) -> Result<Option<Self>, SessionError> {
        let is_dollar_sign = reader.advance_u8().await? == 0x24;
        if is_dollar_sign {
            reader.read_u8().await?;
            let channel_identifier = reader.read_u8().await?;
            let length = reader.read_u16::<BigEndian>().await?;
            return Ok(Some(InterleavedBinaryData {
                channel_identifier,
                length,
            }));
        }
        Ok(None)
    }
}

impl RtspServerSession {
    async fn run(&mut self) -> Result<(), SessionError> {
        loop {
            if let Ok(data) = InterleavedBinaryData::new(&mut self.reader).await {
                match data {
                    Some(a) => {}
                    None => {
                        // self.on_rtp_over_rtsp_message()?;
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
        self.reader.read().await?;

        let data = self.reader.bytes_reader.extract_remaining_bytes();

        if let Some(rtsp_request) = RtspRequest::unmarshal(std::str::from_utf8(&data)?) {
            match rtsp_request.method.as_str() {
                rtsp_method_name::OPTIONS => {
                    let status_code = http::StatusCode::OK;

                    let mut response = Self::gen_response(status_code, &rtsp_request);
                    let public_str = rtsp_method_name::ARRAY.join(",");
                    response.headers.insert("Public".to_string(), public_str);

                    self.send_response(&response)?;
                }
                rtsp_method_name::DESCRIBE => {}
                rtsp_method_name::ANNOUNCE => {
                    if let Some(request_body) = rtsp_request.body {
                        if let Some(sdp) = Sdp::unmarshal(&request_body) {
                            self.sdp = sdp;
                        }
                    }

                    for media in &self.sdp.medias {
                        let media_name = &media.media_type;

                        let media_control =
                            if let Some(media_control_val) = media.attributes.get("control") {
                                media_control_val.clone()
                            } else {
                                String::from("")
                            };

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

    fn on_rtp_over_rtsp_message(&mut self) {}
    fn send_response(&mut self, response: &RtspResponse) -> Result<(), SessionError> {
        self.writer.write(response.marshal().as_bytes())?;

        Ok(())
        //response.
    }
}
