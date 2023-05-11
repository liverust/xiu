pub mod fmtp;
pub mod rtpmap;

use crate::global_trait::TMsgConverter;

use failure::Backtrace;
use rtpmap::RtpMap;
use std::collections::HashMap;

use self::fmtp::Fmtp;

#[derive(Debug, Clone, Default)]
pub struct Bandwidth {
    b_type: String,
    bandwidth: u16,
}

impl TMsgConverter for Bandwidth {
    //   b=AS:284\r\n\
    fn unmarshal(raw_data: &str) -> Option<Self> {
        let mut sdp_bandwidth = Bandwidth::default();

        let parameters: Vec<&str> = raw_data.split(':').collect();
        if let Some(t) = parameters.get(0) {
            sdp_bandwidth.b_type = t.to_string();
        }

        if let Some(bandwidth) = parameters.get(1) {
            if let Ok(bandwidth) = bandwidth.parse::<u16>() {
                sdp_bandwidth.bandwidth = bandwidth;
            }
        }

        Some(sdp_bandwidth)
    }

    fn marshal(&self) -> String {
        format!("{}:{}\r\n", self.b_type, self.bandwidth)
    }
}

/*
v=0
o=- 946685052188730 1 IN IP4 0.0.0.0
s=RTSP/RTP Server
i=playback/robot=040082d087c335e3bd2b/camera=head/timerang1=1533620879-1533620898
t=0 0
a=tool:vlc 0.9.8a
a=type:broadcast
a=control:*
a=range:npt=0-
m=video 20003 RTP/AVP 97
b=RR:0
a=rtpmap:97 H264/90000
a=fmtp:97 profile-level-id=42C01E;packetization-mode=1;sprop-parameter-sets=Z0LAHtkDxWhAAAADAEAAAAwDxYuSAAAAAQ==,aMuMsgAAAAE=
a=control:track1
m=audio 11704 RTP/AVP 96 97 98 0 8 18 101 99 100 */

#[derive(Default, Debug)]
struct SdpMediaInfo {
    media_type: String,
    port: usize,
    protocol: String,
    fmts: Vec<u8>,
    bandwidth: Bandwidth,
    rtpmap: RtpMap,
    fmtp: Option<fmtp::Fmtp>,
    attributes: HashMap<String, String>,
}

// impl std::fmt::Debug for dyn TMsgConverter {
//     fn fmt(&self, fmt: &mut std::fmt::Formatter<'_>) -> Result<(), std::fmt::Error> {
//         write!(fmt, "S2 {{ member: {:?} }}", self.member)
//     }
// }

// impl Default for SdpMediaInfo {
//     fn default() -> Self {
//         Self {
//             fmtp: Box::new(fmtp::UnknownFmtpSdp::default()),
//             ..Default::default()
//         }
//     }
// }

#[derive(Default, Debug)]
struct Sdp {
    raw_string: String,
    version: u16,
    origin: String,
    session: String,
    connection: String,
    timing: String,
    medias: Vec<SdpMediaInfo>,
    attributes: HashMap<String, String>,
}

impl TMsgConverter for SdpMediaInfo {
    //m=audio 11704 RTP/AVP 96 97 98 0 8 18 101 99 100 */
    //m=video 20003 RTP/AVP 97
    fn unmarshal(raw_data: &str) -> Option<Self> {
        let mut sdp_media = SdpMediaInfo::default();
        let parameters: Vec<&str> = raw_data.split(' ').collect();
        let param_len = parameters.len();

        if param_len > 0 {
            sdp_media.media_type = parameters[0].to_string();
        }
        if param_len > 1 {
            if let Ok(port) = parameters[1].parse::<usize>() {
                sdp_media.port = port;
            }
        }
        if param_len > 2 {
            sdp_media.protocol = parameters[2].to_string();
        }

        let mut cur_param_idx = 3;
        while cur_param_idx < param_len {
            if let Ok(fmt) = parameters[cur_param_idx].parse::<u8>() {
                sdp_media.fmts.push(fmt);
            }

            cur_param_idx += 1;
        }
        Some(sdp_media)
    }
    fn marshal(&self) -> String {
        String::default()
    }
}

impl TMsgConverter for Sdp {
    fn unmarshal(raw_data: &str) -> Option<Self> {
        let mut sdp = Sdp::default();
        sdp.raw_string = raw_data.to_string();

        let lines: Vec<&str> = raw_data.split(|c| c == '\r' || c == '\n').collect();
        for line in lines {
            let kv: Vec<&str> = line.trim().splitn(2, '=').collect();
            if kv.len() < 2 {
                log::error!("Sdp current line : {} parse error!", line);
                continue;
            }

            match kv[0] {
                //m=audio 11704 RTP/AVP 96 97 98 0 8 18 101 99 100 */
                //m=video 20003 RTP/AVP 97

                // v=0\r\n\
                // o=- 0 0 IN IP4 127.0.0.1\r\n\
                // s=No Name\r\n\
                // c=IN IP4 127.0.0.1\r\n\
                // t=0 0\r\n\

                // m=video 0 RTP/AVP 96\r\n\
                // b=AS:284\r\n\
                // a=rtpmap:96 H264/90000\r\n\
                // a=fmtp:96 packetization-mode=1; sprop-parameter-sets=Z2QAHqzZQKAv+XARAAADAAEAAAMAMg8WLZY=,aOvjyyLA; profile-level-id=64001E\r\n\
                // a=control:streamid=0\r\n\
                // m=audio 0 RTP/AVP 97\r\n\
                // b=AS:128\r\n\
                // a=rtpmap:97 MPEG4-GENERIC/48000/2\r\n\
                // a=fmtp:97 profile-level-id=1;mode=AAC-hbr;sizelength=13;indexlength=3;indexdeltalength=3; config=119056E500\r\n\
                // a=control:streamid=1\r\n";
                "v" => {
                    if let Ok(version) = kv[1].parse::<u16>() {
                        sdp.version = version;
                    }
                }
                "o" => {
                    sdp.origin = kv[1].to_string();
                }
                "s" => {
                    sdp.session = kv[1].to_string();
                }
                "c" => {
                    sdp.connection = kv[1].to_string();
                }
                "t" => {
                    sdp.timing = kv[1].to_string();
                }
                "m" => {
                    if let Some(sdp_media) = SdpMediaInfo::unmarshal(kv[1]) {
                        sdp.medias.push(sdp_media);
                    }
                }
                "b" => {
                    let sdp_medias_len = sdp.medias.len();
                    if sdp_medias_len == 0 {
                        continue;
                    }
                    if let Some(cur_media) = sdp.medias.get_mut(sdp_medias_len - 1) {
                        cur_media.bandwidth = Bandwidth::unmarshal(kv[1]).unwrap();
                    }
                }
                "a" => {
                    let attribute: Vec<&str> = kv[1].splitn(2, ':').collect();

                    let attr_name = attribute[0];
                    let attr_value = if attribute.len() > 1 {
                        attribute[1]
                    } else {
                        ""
                    };

                    let medias_len = sdp.medias.len();
                    let attributes = if medias_len == 0 {
                        //attributes do not under the 'm' line
                        &mut sdp.attributes
                    } else {
                        if let Some(cur_media) = sdp.medias.get_mut(medias_len - 1) {
                            let parameters: Vec<&str> = kv[1].splitn(2, ':').collect();
                            if parameters.len() == 2 {
                                match parameters[0] {
                                    "rtpmap" => {
                                        if let Some(rtpmap) = RtpMap::unmarshal(parameters[1]) {
                                            cur_media.rtpmap = rtpmap;
                                            continue;
                                        }
                                    }
                                    "fmtp" => {
                                        cur_media.fmtp = Fmtp::new(
                                            &cur_media.rtpmap.encoding_name,
                                            parameters[1],
                                        );
                                        continue;
                                    }
                                    _ => {}
                                }
                            }
                            &mut cur_media.attributes
                        } else {
                            log::error!("should not be here!");
                            continue;
                        }
                    };
                    attributes.insert(attr_name.to_string(), attr_value.to_string());
                }

                _ => {
                    log::info!("not parsed: {}", line);
                }
            }
        }

        Some(sdp)
    }
    fn marshal(&self) -> String {
        String::default()
    }
}

#[cfg(test)]
mod tests {

    use crate::global_trait::TMsgConverter;

    use super::Sdp;
    use indexmap::IndexMap;
    use std::io::{BufRead, BufReader, Read};
    #[test]
    fn test_parse_sdp() {
        let data2 = "ANNOUNCE rtsp://127.0.0.1:5544/stream RTSP/1.0\r\n\
        Content-Type: application/sdp\r\n\
        CSeq: 2\r\n\
        User-Agent: Lavf58.76.100\r\n\
        Content-Length: 500\r\n\
        \r\n\
        v=0\r\n\
        o=- 0 0 IN IP4 127.0.0.1\r\n\
        s=No Name\r\n\
        c=IN IP4 127.0.0.1\r\n\
        t=0 0\r\n\
        a=tool:libavformat 58.76.100\r\n\
        m=video 0 RTP/AVP 96\r\n\
        b=AS:284\r\n\
        a=rtpmap:96 H264/90000\r\n\
        a=fmtp:96 packetization-mode=1; sprop-parameter-sets=Z2QAHqzZQKAv+XARAAADAAEAAAMAMg8WLZY=,aOvjyyLA; profile-level-id=64001E\r\n\
        a=control:streamid=0\r\n\
        m=audio 0 RTP/AVP 97\r\n\
        b=AS:128\r\n\
        a=rtpmap:97 MPEG4-GENERIC/48000/2\r\n\
        a=fmtp:97 profile-level-id=1;mode=AAC-hbr;sizelength=13;indexlength=3;indexdeltalength=3; config=119056E500\r\n\
        a=control:streamid=1\r\n";

        // v=0：SDP版本号，通常为0。
        // o=- 0 0 IN IP4 127.0.0.1：会话的所有者和会话ID，以及会话开始时间和会话结束时间的信息。
        // s=No Name：会话名称或标题。
        // c=IN IP4 127.0.0.1：表示会话数据传输的地址类型(IPv4)和地址(127.0.0.1)。
        // t=0 0：会话时间，包括会话开始时间和结束时间，这里的值都是0，表示会话没有预定义的结束时间。
        // a=tool:libavformat 58.76.100：会话所使用的工具或软件名称和版本号。

        // m=video 0 RTP/AVP 96：媒体类型(video或audio)、媒体格式(RTP/AVP)、媒体格式编号(96)和媒体流的传输地址。
        // b=AS:284：视频流所使用的带宽大小。
        // a=rtpmap:96 H264/90000：视频流所使用的编码方式(H.264)和时钟频率(90000)。
        // a=fmtp:96 packetization-mode=1; sprop-parameter-sets=Z2QAHqzZQKAv+XARAAADAAEAAAMAMg8WLZY=,aOvjyyLA; profile-level-id=64001E：视频流的格式参数，如分片方式、SPS和PPS等。
        // a=control:streamid=0：指定视频流的流ID。

        // m=audio 0 RTP/AVP 97：媒体类型(audio)、媒体格式(RTP/AVP)、媒体格式编号(97)和媒体流的传输地址。
        // b=AS:128：音频流所使用的带宽大小。
        // a=rtpmap:97 MPEG4-GENERIC/48000/2：音频流所使用的编码方式(MPEG4-GENERIC)、采样率(48000Hz)、和通道数(2)。
        // a=fmtp:97 profile-level-id=1;mode=AAC-hbr;sizelength=13;indexlength=3;indexdeltalength=3; config=119056E500：音频流的格式参数，如编码方式、采样长度、索引长度等。
        // a=control:streamid=1：指定音频流的流ID。

        if let Some(sdp) = Sdp::unmarshal(data2) {
            println!("sdp : {:?}", sdp);
        }
    }
}
