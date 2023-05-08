pub mod fmtp;
pub mod rtpmap;

use fmtp::Fmtp;
use rtpmap::RtpMap;
use std::collections::HashMap;

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
#[derive(Debug, Clone, Default)]
struct SdpMedia {
    media_type: String,
    port: usize,
    protocol: String,
    fmts: Vec<u8>,
    attributes: HashMap<String, String>,
    control: String,
}

#[derive(Debug, Clone, Default)]
struct Sdp {
    medias: Vec<SdpMedia>,
    attributes: HashMap<String, String>,
}

impl SdpMedia {
    //m=audio 11704 RTP/AVP 96 97 98 0 8 18 101 99 100 */
    //m=video 20003 RTP/AVP 97
    fn parse(&mut self, raw_data: &str) {
        let parameters: Vec<&str> = raw_data.split(' ').collect();
        let param_len = parameters.len();

        if param_len > 0 {
            self.media_type = parameters[0].to_string();
        }
        if param_len > 1 {
            if let Ok(port) = parameters[1].parse::<usize>() {
                self.port = port;
            }
        }
        if param_len > 2 {
            self.protocol = parameters[2].to_string();
        }

        let mut cur_param_idx = 3;
        while cur_param_idx < param_len {
            if let Ok(fmt) = parameters[cur_param_idx].parse::<u8>() {
                self.fmts.push(fmt);
            }

            cur_param_idx += 1;
        }
    }
}

impl Sdp {
    fn parse(&mut self, raw_data: &str) {
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
                "m" => {
                    let mut sdp_media = SdpMedia::default();
                    sdp_media.parse(kv[1]);
                    self.medias.push(sdp_media);
                }
                "a" => {
                    let attribute: Vec<&str> = kv[1].splitn(2, ':').collect();
                    let attr_name = attribute[0];
                    let attr_value = if attribute.len() > 1 {
                        attribute[1]
                    } else {
                        ""
                    };
                    let medias_len = self.medias.len();
                    let attributes = if medias_len == 0 {
                        &mut self.attributes
                    } else {
                        if let Some(cur_media) = self.medias.get_mut(medias_len - 1) {
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
    }
}
