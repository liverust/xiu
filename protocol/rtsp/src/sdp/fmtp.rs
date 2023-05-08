use bytes::BytesMut;

pub trait Fmtp {
    fn parse(&mut self, raw_data: String);
}

#[derive(Debug, Clone, Default)]
struct H264Fmtp {
    packetization_mode: u8,
    profile_level_id: BytesMut,
    sps: BytesMut,
    pps: BytesMut,
}
#[derive(Debug, Clone, Default)]
struct H265Fmtp {
    vps: BytesMut,
    sps: BytesMut,
    pps: BytesMut,
}
#[derive(Debug, Clone, Default)]
struct Mpeg4Fmtp {
    asc: BytesMut,
    profile_level_id: BytesMut,
    mode: String,
    size_length: u16,
    index_length: u16,
    index_delta_length: u16,
}
#[derive(Default)]
struct UnknownFmtpSdp {}

fn create_fmtp_sdp_parser(n: &str) -> Box<dyn Fmtp> {
    match n {
        "h264" => Box::new(H264Fmtp::default()),
        "h265" => Box::new(H265Fmtp::default()),
        "mpeg4-generic" => Box::new(Mpeg4Fmtp::default()),
        _ => Box::new(UnknownFmtpSdp::default()),
    }
}

// a=fmtp:96 packetization-mode=1; sprop-parameter-sets=Z2QAFqyyAUBf8uAiAAADAAIAAAMAPB4sXJA=,aOvDyyLA; profile-level-id=640016
impl Fmtp for H264Fmtp {
    fn parse(&mut self, raw_data: String) {
        let eles: Vec<&str> = raw_data.splitn(2, ' ').collect();
        if eles.len() < 2 {
            log::warn!("H264FmtpSdp parse err: {}", raw_data);
            return;
        }
        let parameters: Vec<&str> = eles[1].split(';').collect();

        for parameter in parameters {
            let kv: Vec<&str> = parameter.trim().splitn(2, '=').collect();
            if kv.len() < 2 {
                log::warn!("H264FmtpSdp parse key=value err: {}", parameter);
                continue;
            }
            match kv[0] {
                "packetization-mode" => {
                    if let Ok(packetization_mode) = kv[1].parse::<u8>() {
                        self.packetization_mode = packetization_mode;
                    }
                }
                "sprop-parameter-sets" => {
                    let spspps: Vec<&str> = kv[1].split(',').collect();
                    self.sps = spspps[0].into();
                    self.pps = spspps[1].into();
                }
                "profile-level-id" => {
                    self.profile_level_id = kv[1].into();
                }
                _ => {
                    log::info!("not parsed: {}", kv[0])
                }
            }
        }
    }
}

impl Fmtp for H265Fmtp {
    //"a=fmtp:96 sprop-vps=QAEMAf//AWAAAAMAkAAAAwAAAwA/ugJA; sprop-sps=QgEBAWAAAAMAkAAAAwAAAwA/oAUCAXHy5bpKTC8BAQAAAwABAAADAA8I; sprop-pps=RAHAc8GJ"
    fn parse(&mut self, raw_data: String) {
        let eles: Vec<&str> = raw_data.splitn(2, ' ').collect();
        if eles.len() < 2 {
            log::warn!("H265FmtpSdp parse err: {}", raw_data);
            return;
        }
        let parameters: Vec<&str> = eles[1].split(';').collect();

        for parameter in parameters {
            let kv: Vec<&str> = parameter.trim().splitn(2, '=').collect();
            if kv.len() < 2 {
                log::warn!("H265FmtpSdp parse key=value err: {}", parameter);
                continue;
            }

            match kv[0] {
                "sprop-vps" => {
                    self.vps = kv[1].into();
                }
                "sprop-sps" => {
                    self.sps = kv[1].into();
                }
                "sprop-pps" => {
                    self.pps = kv[1].into();
                }
                _ => {
                    log::info!("not parsed: {}", kv[0])
                }
            }
        }
    }
}

impl Fmtp for Mpeg4Fmtp {
    //a=fmtp:97 profile-level-id=1;mode=AAC-hbr;sizelength=13;indexlength=3;indexdeltalength=3; config=121056e500
    fn parse(&mut self, raw_data: String) {
        let eles: Vec<&str> = raw_data.splitn(2, ' ').collect();
        if eles.len() < 2 {
            log::warn!("Mpeg4FmtpSdp parse err: {}", raw_data);
            return;
        }
        let parameters: Vec<&str> = eles[1].split(';').collect();

        for parameter in parameters {
            let kv: Vec<&str> = parameter.trim().splitn(2, '=').collect();
            if kv.len() < 2 {
                log::warn!("Mpeg4FmtpSdp parse key=value err: {}", parameter);
                continue;
            }
            match kv[0].to_lowercase().as_str() {
                "mode" => {
                    self.mode = kv[1].to_string();
                }
                "config" => {
                    self.asc = kv[1].into();
                }
                "profile-level-id" => {
                    self.profile_level_id = kv[1].into();
                }
                "sizelength" => {
                    if let Ok(size_length) = kv[1].parse::<u16>() {
                        self.size_length = size_length;
                    }
                }
                "indexlength" => {
                    if let Ok(index_length) = kv[1].parse::<u16>() {
                        self.index_length = index_length;
                    }
                }
                "indexdeltalength" => {
                    if let Ok(index_delta_length) = kv[1].parse::<u16>() {
                        self.index_delta_length = index_delta_length;
                    }
                }
                _ => {
                    log::info!("not parsed: {}", kv[0])
                }
            }
        }
    }
}

impl Fmtp for UnknownFmtpSdp {
    fn parse(&mut self, raw_data: String) {}
}

#[cfg(test)]
mod tests {

    use super::Fmtp;
    use super::H264Fmtp;
    use super::H265Fmtp;
    use super::Mpeg4Fmtp;

    #[test]
    fn test_parse_h264fmtpsdp() {
        let mut parser = H264Fmtp::default();

        parser.parse("a=fmtp:96 packetization-mode=1; sprop-parameter-sets=Z2QAFqyyAUBf8uAiAAADAAIAAAMAPB4sXJA=,aOvDyyLA; profile-level-id=640016".to_string());

        println!(" parser: {:?}", parser);

        assert_eq!(parser.packetization_mode, 1);
        assert_eq!(parser.profile_level_id, "640016");
        assert_eq!(parser.sps, "Z2QAFqyyAUBf8uAiAAADAAIAAAMAPB4sXJA=");
        assert_eq!(parser.pps, "aOvDyyLA");

        let mut parser2 = H264Fmtp::default();

        parser2.parse("a=fmtp:96 packetization-mode=1;\nsprop-parameter-sets=Z2QAFqyyAUBf8uAiAAADAAIAAAMAPB4sXJA=,aOvDyyLA;\nprofile-level-id=640016".to_string());

        println!(" parser: {:?}", parser2);

        assert_eq!(parser2.packetization_mode, 1);
        assert_eq!(parser2.profile_level_id, "640016");
        assert_eq!(parser2.sps, "Z2QAFqyyAUBf8uAiAAADAAIAAAMAPB4sXJA=");
        assert_eq!(parser2.pps, "aOvDyyLA");
    }
    #[test]
    fn test_parse_h265fmtpsdp() {
        let mut parser = H265Fmtp::default();

        parser.parse("a=fmtp:96 sprop-vps=QAEMAf//AWAAAAMAkAAAAwAAAwA/ugJA; sprop-sps=QgEBAWAAAAMAkAAAAwAAAwA/oAUCAXHy5bpKTC8BAQAAAwABAAADAA8I; sprop-pps=RAHAc8GJ".to_string());

        println!(" parser: {:?}", parser);

        assert_eq!(parser.vps, "QAEMAf//AWAAAAMAkAAAAwAAAwA/ugJA");
        assert_eq!(
            parser.sps,
            "QgEBAWAAAAMAkAAAAwAAAwA/oAUCAXHy5bpKTC8BAQAAAwABAAADAA8I"
        );
        assert_eq!(parser.pps, "RAHAc8GJ");
    }

    #[test]
    fn test_parse_mpeg4fmtpsdp() {
        let mut parser = Mpeg4Fmtp::default();

        parser.parse("a=fmtp:97 profile-level-id=1;mode=AAC-hbr;sizelength=13;indexlength=3;indexdeltalength=23; config=121056e500".to_string());

        println!(" parser: {:?}", parser);

        assert_eq!(parser.asc, "121056e500");
        assert_eq!(parser.profile_level_id, "1");
        assert_eq!(parser.mode, "AAC-hbr");
        assert_eq!(parser.size_length, 13);
        assert_eq!(parser.index_length, 3);
        assert_eq!(parser.index_delta_length, 23);
    }
}
