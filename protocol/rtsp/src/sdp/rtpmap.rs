use crate::global_trait::TMsgConverter;

#[derive(Debug, Clone, Default)]
pub struct RtpMap {
    payload_type: u16,
    pub encoding_name: String,
    clock_rate: u32,
    encoding_param: String,
}

impl TMsgConverter for RtpMap {
    fn unmarshal(raw_data: &str) -> Option<Self> {
        let mut rtpmap = RtpMap::default();

        let parts: Vec<&str> = raw_data.split(' ').collect();

        if let Ok(payload_type) = parts[0].parse::<u16>() {
            rtpmap.payload_type = payload_type;
        }

        let second_parts: Vec<&str> = parts[1].split('/').collect();
        let second_part_size = second_parts.len();

        if second_part_size > 0 {
            rtpmap.encoding_name = second_parts[0].to_string();
        }
        if second_part_size > 1 {
            if let Ok(clock_rate) = second_parts[1].parse::<u32>() {
                rtpmap.clock_rate = clock_rate;
            }
        }
        if second_part_size > 2 {
            rtpmap.encoding_param = second_parts[2].to_string();
        }

        Some(rtpmap)
    }

    fn marshal(&self) -> String {
        String::default()
    }
}

#[cfg(test)]
mod tests {

    use crate::global_trait::TMsgConverter;

    use super::RtpMap;

    #[test]
    fn test_parse_rtpmap() {
        let mut parser = RtpMap::unmarshal("97 MPEG4-GENERIC/44100/2").unwrap();

        println!(" parser: {:?}", parser);

        assert_eq!(parser.payload_type, 97);
        assert_eq!(parser.encoding_name, "MPEG4-GENERIC".to_string());
        assert_eq!(parser.clock_rate, 44100);
        assert_eq!(parser.encoding_param, "2".to_string());

        let mut parser2 = RtpMap::unmarshal("96 H264/90000").unwrap();

        println!(" parser2: {:?}", parser2);

        assert_eq!(parser2.payload_type, 96);
        assert_eq!(parser2.encoding_name, "H264".to_string());
        assert_eq!(parser2.clock_rate, 90000);
        assert_eq!(parser2.encoding_param, "".to_string());
    }
}
