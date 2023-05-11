use crate::global_trait::TMsgConverter;
use bytes::BytesMut;

// pub trait Fmtp: TMsgConverter {}

#[derive(Debug, Clone, Default)]
pub struct H264Fmtp {
    packetization_mode: u8,
    profile_level_id: BytesMut,
    sps: BytesMut,
    pps: BytesMut,
}
#[derive(Debug, Clone, Default)]
pub struct H265Fmtp {
    vps: BytesMut,
    sps: BytesMut,
    pps: BytesMut,
}
#[derive(Debug, Clone, Default)]
pub struct Mpeg4Fmtp {
    asc: BytesMut,
    profile_level_id: BytesMut,
    mode: String,
    size_length: u16,
    index_length: u16,
    index_delta_length: u16,
}
#[derive(Debug, Clone)]
pub enum Fmtp {
    H264(H264Fmtp),
    H265(H265Fmtp),
    Mpeg4(Mpeg4Fmtp),
}

impl Fmtp {
    pub fn new(codec: &str, raw_data: &str) -> Option<Fmtp> {
        match codec.to_lowercase().as_str() {
            "h264" => {
                if let Some(h264_fmtp) = H264Fmtp::unmarshal(raw_data) {
                    return Some(Fmtp::H264(h264_fmtp));
                }
            }
            "h265" => {
                if let Some(h265_fmtp) = H265Fmtp::unmarshal(raw_data) {
                    return Some(Fmtp::H265(h265_fmtp));
                }
            }
            "mpeg4-generic" => {
                if let Some(mpeg4_fmtp) = Mpeg4Fmtp::unmarshal(raw_data) {
                    return Some(Fmtp::Mpeg4(mpeg4_fmtp));
                }
            }
            _ => {}
        }
        None
    }
}

// a=fmtp:96 packetization-mode=1; sprop-parameter-sets=Z2QAFqyyAUBf8uAiAAADAAIAAAMAPB4sXJA=,aOvDyyLA; profile-level-id=640016
impl TMsgConverter for H264Fmtp {
    fn unmarshal(raw_data: &str) -> Option<Self> {
        let mut h264_fmtp = H264Fmtp::default();
        let eles: Vec<&str> = raw_data.splitn(2, ' ').collect();
        if eles.len() < 2 {
            log::warn!("H264FmtpSdp parse err: {}", raw_data);
            return None;
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
                        h264_fmtp.packetization_mode = packetization_mode;
                    }
                }
                "sprop-parameter-sets" => {
                    let spspps: Vec<&str> = kv[1].split(',').collect();
                    h264_fmtp.sps = spspps[0].into();
                    h264_fmtp.pps = spspps[1].into();
                }
                "profile-level-id" => {
                    h264_fmtp.profile_level_id = kv[1].into();
                }
                _ => {
                    log::info!("not parsed: {}", kv[0])
                }
            }
        }

        Some(h264_fmtp)
    }
    fn marshal(&self) -> String {
        String::default()
    }
}

impl TMsgConverter for H265Fmtp {
    //"a=fmtp:96 sprop-vps=QAEMAf//AWAAAAMAkAAAAwAAAwA/ugJA; sprop-sps=QgEBAWAAAAMAkAAAAwAAAwA/oAUCAXHy5bpKTC8BAQAAAwABAAADAA8I; sprop-pps=RAHAc8GJ"
    fn unmarshal(raw_data: &str) -> Option<Self> {
        let mut h265_fmtp = H265Fmtp::default();
        let eles: Vec<&str> = raw_data.splitn(2, ' ').collect();
        if eles.len() < 2 {
            log::warn!("H265FmtpSdp parse err: {}", raw_data);
            return None;
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
                    h265_fmtp.vps = kv[1].into();
                }
                "sprop-sps" => {
                    h265_fmtp.sps = kv[1].into();
                }
                "sprop-pps" => {
                    h265_fmtp.pps = kv[1].into();
                }
                _ => {
                    log::info!("not parsed: {}", kv[0])
                }
            }
        }

        Some(h265_fmtp)
    }
    fn marshal(&self) -> String {
        String::default()
    }
}

impl TMsgConverter for Mpeg4Fmtp {
    //a=fmtp:97 profile-level-id=1;mode=AAC-hbr;sizelength=13;indexlength=3;indexdeltalength=3; config=121056e500
    fn unmarshal(raw_data: &str) -> Option<Self> {
        let mut mpeg4_fmtp = Mpeg4Fmtp::default();
        let eles: Vec<&str> = raw_data.splitn(2, ' ').collect();
        if eles.len() < 2 {
            log::warn!("Mpeg4FmtpSdp parse err: {}", raw_data);
            return None;
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
                    mpeg4_fmtp.mode = kv[1].to_string();
                }
                "config" => {
                    mpeg4_fmtp.asc = kv[1].into();
                }
                "profile-level-id" => {
                    mpeg4_fmtp.profile_level_id = kv[1].into();
                }
                "sizelength" => {
                    if let Ok(size_length) = kv[1].parse::<u16>() {
                        mpeg4_fmtp.size_length = size_length;
                    }
                }
                "indexlength" => {
                    if let Ok(index_length) = kv[1].parse::<u16>() {
                        mpeg4_fmtp.index_length = index_length;
                    }
                }
                "indexdeltalength" => {
                    if let Ok(index_delta_length) = kv[1].parse::<u16>() {
                        mpeg4_fmtp.index_delta_length = index_delta_length;
                    }
                }
                _ => {
                    log::info!("not parsed: {}", kv[0])
                }
            }
        }

        Some(mpeg4_fmtp)
    }
    fn marshal(&self) -> String {
        String::default()
    }
}

#[cfg(test)]
mod tests {

    use super::H264Fmtp;
    use super::H265Fmtp;
    use super::Mpeg4Fmtp;
    use crate::global_trait::TMsgConverter;

    #[test]
    fn test_parse_h264fmtpsdp() {
        let parser =  H264Fmtp::unmarshal("a=fmtp:96 packetization-mode=1; sprop-parameter-sets=Z2QAFqyyAUBf8uAiAAADAAIAAAMAPB4sXJA=,aOvDyyLA; profile-level-id=640016").unwrap();

        println!(" parser: {:?}", parser);

        assert_eq!(parser.packetization_mode, 1);
        assert_eq!(parser.profile_level_id, "640016");
        assert_eq!(parser.sps, "Z2QAFqyyAUBf8uAiAAADAAIAAAMAPB4sXJA=");
        assert_eq!(parser.pps, "aOvDyyLA");

        let parser2 = H264Fmtp::unmarshal("a=fmtp:96 packetization-mode=1;\nsprop-parameter-sets=Z2QAFqyyAUBf8uAiAAADAAIAAAMAPB4sXJA=,aOvDyyLA;\nprofile-level-id=640016").unwrap();

        println!(" parser: {:?}", parser2);

        assert_eq!(parser2.packetization_mode, 1);
        assert_eq!(parser2.profile_level_id, "640016");
        assert_eq!(parser2.sps, "Z2QAFqyyAUBf8uAiAAADAAIAAAMAPB4sXJA=");
        assert_eq!(parser2.pps, "aOvDyyLA");
    }
    #[test]
    fn test_parse_h265fmtpsdp() {
        let parser = H265Fmtp::unmarshal("a=fmtp:96 sprop-vps=QAEMAf//AWAAAAMAkAAAAwAAAwA/ugJA; sprop-sps=QgEBAWAAAAMAkAAAAwAAAwA/oAUCAXHy5bpKTC8BAQAAAwABAAADAA8I; sprop-pps=RAHAc8GJ").unwrap();

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
        let parser = Mpeg4Fmtp::unmarshal("a=fmtp:97 profile-level-id=1;mode=AAC-hbr;sizelength=13;indexlength=3;indexdeltalength=23; config=121056e500").unwrap();

        println!(" parser: {:?}", parser);

        assert_eq!(parser.asc, "121056e500");
        assert_eq!(parser.profile_level_id, "1");
        assert_eq!(parser.mode, "AAC-hbr");
        assert_eq!(parser.size_length, 13);
        assert_eq!(parser.index_length, 3);
        assert_eq!(parser.index_delta_length, 23);
    }
}
