use http::Request;
use http::StatusCode;
use indexmap::IndexMap;
use std::io::{Read, Result};
use std::net::TcpStream;

use crate::global_trait::TMsgConverter;

#[derive(Debug, Clone, Default)]
pub struct RtspRequest {
    pub method: String,
    pub url: String,
    pub version: String,
    pub headers: IndexMap<String, String>,
    pub body: Option<String>,
}

impl TMsgConverter for RtspRequest {
    fn unmarshal(request_data: &str) -> Option<Self> {
        let mut rtsp_request = RtspRequest::default();
        let header_end_idx = if let Some(idx) = request_data.find("\r\n\r\n") {
            let data_except_body = &request_data[..idx];
            let mut lines = data_except_body.lines();
            //parse the first line
            if let Some(request_first_line) = lines.next() {
                let mut fields = request_first_line.split_ascii_whitespace();
                if let Some(method) = fields.next() {
                    rtsp_request.method = method.to_string();
                }
                if let Some(url) = fields.next() {
                    rtsp_request.url = url.to_string();
                }
                if let Some(version) = fields.next() {
                    rtsp_request.version = version.to_string();
                }
            }
            //parse headers
            for line in lines {
                if let Some(index) = line.find(": ") {
                    let name = line[..index].to_string();
                    let value = line[index + 2..].to_string();
                    rtsp_request.headers.insert(name, value);
                }
            }
            idx + 4
        } else {
            return None;
        };

        if request_data.len() > header_end_idx {
            //parse body
            rtsp_request.body = Some(request_data[header_end_idx..].to_string());
        }

        Some(rtsp_request)
    }
    fn marshal(&self) -> String {
        let mut request_str = format!("{} {} {}\r\n", self.method, self.url, self.version);
        for (header_name, header_value) in &self.headers {
            if header_name != &"Content-Length".to_string() {
                request_str += &format!("{}: {}\r\n", header_name, header_value);
            }
        }
        if let Some(body) = &self.body {
            request_str += &format!("Content-Length: {}\r\n", body.len());
        }
        request_str += "\r\n";
        if let Some(body) = &self.body {
            request_str += body;
        }
        request_str
    }
}

#[derive(Debug, Clone, Default)]
pub struct RtspResponse {
    pub version: String,
    pub status_code: u16,
    pub reason_phrase: String,
    pub headers: IndexMap<String, String>,
    pub body: Option<String>,
}

impl TMsgConverter for RtspResponse {
    fn unmarshal(request_data: &str) -> Option<Self> {
        let mut rtsp_response = RtspResponse::default();
        let header_end_idx = if let Some(idx) = request_data.find("\r\n\r\n") {
            let data_except_body = &request_data[..idx];
            let mut lines = data_except_body.lines();
            //parse the first line
            if let Some(request_first_line) = lines.next() {
                let mut fields = request_first_line.split_ascii_whitespace();

                if let Some(version) = fields.next() {
                    rtsp_response.version = version.to_string();
                }
                if let Some(status) = fields.next() {
                    if let Ok(status) = status.parse::<u16>() {
                        rtsp_response.status_code = status;
                    }
                }
                if let Some(reason_phrase) = fields.next() {
                    rtsp_response.reason_phrase = reason_phrase.to_string();
                }
            }
            //parse headers
            for line in lines {
                if let Some(index) = line.find(": ") {
                    let name = line[..index].to_string();
                    let value = line[index + 2..].to_string();
                    rtsp_response.headers.insert(name, value);
                }
            }
            idx + 4
        } else {
            return None;
        };

        if request_data.len() > header_end_idx {
            //parse body
            rtsp_response.body = Some(request_data[header_end_idx..].to_string());
        }

        Some(rtsp_response)
    }

    fn marshal(&self) -> String {
        let mut response_str = format!(
            "{} {} {}\r\n",
            self.version, self.status_code, self.reason_phrase
        );
        for (header_name, header_value) in &self.headers {
            if header_name != &"Content-Length".to_string() {
                response_str += &format!("{}: {}\r\n", header_name, header_value);
            }
        }
        if let Some(body) = &self.body {
            response_str += &format!("Content-Length: {}\r\n", body.len());
        }
        response_str += "\r\n";
        if let Some(body) = &self.body {
            response_str += body;
        }
        response_str
    }
}

#[cfg(test)]
mod tests {

    use crate::http::parser::TMsgConverter;

    use super::RtspRequest;

    use indexmap::IndexMap;
    use std::io::{BufRead, BufReader, Read};

    pub fn parse_request_bytes(data: &[u8]) -> Option<RtspRequest> {
        let mut reader = BufReader::new(data);
        // read the first line to get the request method, URL and version
        let mut first_line = String::new();
        if let Ok(size) = reader.read_line(&mut first_line) {
            if size == 0 {
                return None;
            }
            let mut fields = first_line.trim_end().split_ascii_whitespace();
            let method = fields.next()?.to_string();
            let url = fields.next()?.to_string();
            let version = fields.next()?.to_string();
            // read headers
            let headers = read_headers(&mut reader)?;
            // read body if there is any
            let mut body = String::new();
            reader.read_to_string(&mut body).ok();
            Some(RtspRequest {
                method,
                url,
                version,
                headers,
                body: if body.is_empty() { None } else { Some(body) },
            })
        } else {
            None
        }
    }

    fn read_headers(reader: &mut dyn BufRead) -> Option<IndexMap<String, String>> {
        let mut headers = IndexMap::new();
        loop {
            let mut line = String::new();
            match reader.read_line(&mut line) {
                Ok(0) => break,
                Ok(_) => {
                    if let Some(index) = line.find(": ") {
                        let name = line[..index].to_string();
                        let value = line[index + 2..].trim().to_string();
                        headers.insert(name, value);
                    }
                }
                Err(_) => return None,
            }
        }
        Some(headers)
    }

    #[test]
    fn test_parse_rtsp_request_chatgpt() {
        let data1 = "ANNOUNCE rtsp://127.0.0.1:5544/stream RTSP/1.0\r\n\
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
        a=rtpmap:96 H264/90000
        a=fmtp:96 packetization-mode=1; sprop-parameter-sets=Z2QAHqzZQKAv+XARAAADAAEAAAMAMg8WLZY=,aOvjyyLA; profile-level-id=64001E\r\n\
        a=control:streamid=0\r\n\
        m=audio 0 RTP/AVP 97\r\n\
        b=AS:128\r\n\
        a=rtpmap:97 MPEG4-GENERIC/48000/2\r\n\
        a=fmtp:97 profile-level-id=1;mode=AAC-hbr;sizelength=13;indexlength=3;indexdeltalength=3; config=119056E500\r\n\
        a=control:streamid=1\r\n";

        if let Some(request) = parse_request_bytes(&data1.as_bytes()) {
            println!(" parser: {:?}", request);
        }
    }

    #[test]
    fn test_parse_rtsp_request() {
        let data1 = "SETUP rtsp://127.0.0.1:5544/stream/streamid=0 RTSP/1.0\r\n\
        Transport: RTP/AVP/TCP;unicast;interleaved=0-1;mode=record\r\n\
        CSeq: 3\r\n\
        User-Agent: Lavf58.76.100\r\n\
        \r\n";

        if let Some(parser) = RtspRequest::unmarshal(data1) {
            println!(" parser: {:?}", parser);
            let marshal_result = parser.marshal();
            print!("marshal result: =={}==", marshal_result);
            assert_eq!(data1, marshal_result);
        }

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

        if let Some(parser) = RtspRequest::unmarshal(data2) {
            println!(" parser: {:?}", parser);
            let marshal_result = parser.marshal();
            print!("marshal result: =={}==", marshal_result);
            assert_eq!(data2, marshal_result);
        }
    }

    #[test]
    fn test_http_status_code() {
        let stats_code = http::StatusCode::OK;

        println!(
            "stats_code: {}, {}",
            stats_code.canonical_reason().unwrap(),
            stats_code.as_u16()
        )
    }
}
