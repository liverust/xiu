use super::global_trait::TMsgConverter;
use super::rtsp_utils::scanf;

#[derive(Debug, Clone, Default, PartialEq)]

pub enum CastType {
    Multicast,
    #[default]
    Unicast,
}
#[derive(Debug, Clone, Default, PartialEq)]
pub enum ProtocolType {
    #[default]
    TCP,
    UDP,
}
#[derive(Debug, Clone, Default)]
pub struct RtspTransport {
    cast_type: CastType,
    protocol_type: ProtocolType,
    interleaved: [u8; 2],
    transport_mod: String,
    client_port: [usize; 2],
    server_port: [usize; 2],
    ssrc: u32,
}

impl TMsgConverter for RtspTransport {
    fn unmarshal(raw_data: &str) -> Option<Self> {
        let mut rtsp_transport = RtspTransport::default();

        let param_parts: Vec<&str> = raw_data.split(';').collect();
        for part in param_parts {
            let kv: Vec<&str> = part.split('=').collect();
            match kv[0] {
                "RTP/AVP/TCP" => {
                    rtsp_transport.protocol_type = ProtocolType::TCP;
                }
                "RTP/AVP/UDP" | "RTP/AVP" => {
                    rtsp_transport.protocol_type = ProtocolType::UDP;
                }
                "unicast" => {
                    rtsp_transport.cast_type = CastType::Unicast;
                }
                "multicast" => {
                    rtsp_transport.cast_type = CastType::Multicast;
                }
                "mode" => {
                    rtsp_transport.transport_mod = kv[1].to_string();
                }
                "client_port" => {
                    let ports = scanf!(kv[1], '-', usize, usize);
                    if let Some(port) = ports.0 {
                        rtsp_transport.client_port[0] = port;
                    }
                    if let Some(port) = ports.1 {
                        rtsp_transport.client_port[1] = port;
                    }
                }
                "server_port" => {
                    let ports = scanf!(kv[1], '-', usize, usize);
                    if let Some(port) = ports.0 {
                        rtsp_transport.server_port[0] = port;
                    }
                    if let Some(port) = ports.1 {
                        rtsp_transport.server_port[1] = port;
                    }
                }
                "interleaved" => {
                    let vals = scanf!(kv[1], '-', u8, u8);
                    if let Some(val) = vals.0 {
                        rtsp_transport.interleaved[0] = val;
                    }
                    if let Some(val) = vals.1 {
                        rtsp_transport.interleaved[1] = val;
                    }
                }
                "ssrc" => {
                    if let Ok(ssrc) = kv[1].parse::<u32>() {
                        rtsp_transport.ssrc = ssrc;
                    }
                }

                _ => {}
            }
        }

        Some(rtsp_transport)
    }

    fn marshal(&self) -> String {
        String::default()
    }
}

#[cfg(test)]
mod tests {

    use crate::global_trait::TMsgConverter;

    use super::CastType;
    use super::ProtocolType;
    use super::RtspTransport;

    #[test]
    fn test_parse_transport() {
        let parser = RtspTransport::unmarshal(
            "RTP/AVP;unicast;client_port=8000-8001;server_port=9000-9001;ssrc=1234;interleaved=0-1;mode=record",
        ).unwrap();

        println!(" parser: {:?}", parser);

        assert_eq!(parser.cast_type, CastType::Unicast);
        assert_eq!(parser.protocol_type, ProtocolType::UDP);
        assert_eq!(parser.interleaved, [0, 1]);
        assert_eq!(parser.transport_mod, "record".to_string());
        assert_eq!(parser.client_port, [8000, 8001]);
        assert_eq!(parser.server_port, [9000, 9001]);
        assert_eq!(parser.ssrc, 1234);
    }
}
