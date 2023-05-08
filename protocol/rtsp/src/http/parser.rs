use http::Request;
use http::StatusCode;
use std::collections::HashMap;
use std::io::{Read, Result};
use std::net::TcpStream;

const BUFFER_SIZE: usize = 8192;

#[derive(Debug, Clone, Default)]
pub struct RtspRequest {
    pub method: String,
    pub url: String,
    pub version: String,
    pub headers: HashMap<String, String>,
    pub body: Option<String>,
}

impl RtspRequest {
    fn unmarshal(&mut self, request_data: &str) {
        let header_end_idx = if let Some(idx) = request_data.find("\r\n\r\n") {
            let data_except_body = &request_data[..idx];
            let mut lines = data_except_body.lines();
            //parse the first line
            if let Some(request_first_line) = lines.next() {
                let mut fields = request_first_line.split_ascii_whitespace();
                if let Some(method) = fields.next() {
                    self.method = method.to_string();
                }
                if let Some(url) = fields.next() {
                    self.url = url.to_string();
                }
                if let Some(version) = fields.next() {
                    self.version = version.to_string();
                }
            }
            //parse headers
            for line in lines {
                if let Some(index) = line.find(": ") {
                    let name = line[..index].to_string();
                    let value = line[index + 2..].to_string();
                    self.headers.insert(name, value);
                }
            }
            idx + 4
        } else {
            return;
        };

        if request_data.len() > header_end_idx {
            //parse body
            self.body = Some(request_data[header_end_idx..].to_string());
        }
    }
    fn marshal(&mut self) -> String {
        let mut request_str = format!("{} {} {}\r\n", self.method, self.url, self.version);
        for (header_name, header_value) in &self.headers {
            request_str += &format!("{}: {}\r\n", header_name, header_value);
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

struct RtspResponse {
    version: String,
    status_code: u16,
    reason_phrase: String,
    headers: HashMap<String, String>,
    body: Option<String>,
}

impl RtspResponse {
    fn unmarshal(&mut self, request_data: &str) {
        let header_end_idx = if let Some(idx) = request_data.find("\r\n\r\n") {
            let data_except_body = &request_data[..idx];
            let mut lines = data_except_body.lines();
            //parse the first line
            if let Some(request_first_line) = lines.next() {
                let mut fields = request_first_line.split_ascii_whitespace();

                if let Some(version) = fields.next() {
                    self.version = version.to_string();
                }
                if let Some(status) = fields.next() {
                    if let Ok(status) = status.parse::<u16>() {
                        self.status_code = status;
                    }
                }
                if let Some(reason_phrase) = fields.next() {
                    self.reason_phrase = reason_phrase.to_string();
                }
            }
            //parse headers
            for line in lines {
                if let Some(index) = line.find(": ") {
                    let name = line[..index].to_string();
                    let value = line[index + 2..].to_string();
                    self.headers.insert(name, value);
                }
            }
            idx + 4
        } else {
            return;
        };

        if request_data.len() > header_end_idx {
            //parse body
            self.body = Some(request_data[header_end_idx..].to_string());
        }
    }

    pub fn marshal(&self) -> String {
        let mut response_str = format!(
            "{} {} {}\r\n",
            self.version, self.status_code, self.reason_phrase
        );
        for (header_name, header_value) in &self.headers {
            response_str += &format!("{}: {}\r\n", header_name, header_value);
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

    use super::RtspRequest;

    #[test]
    fn test_parse_rtsp_request() {
        let mut parser = RtspRequest::default();
        let data1 = "SETUP rtsp://127.0.0.1:5544/stream/streamid=0 RTSP/1.0\r\n\
        Transport: RTP/AVP/TCP;unicast;interleaved=0-1;mode=record\r\n\
        CSeq: 3\r\n\
        User-Agent: Lavf58.76.100\r\n\
        \r\n";

        parser.unmarshal(data1);
        println!(" parser: {:?}", parser);

        let marshal_result = parser.marshal();
        print!("marshal result: =={}==", marshal_result);

        let mut parser2 = RtspRequest::default();

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
        a=rtpmap:96 H264/90000
        a=fmtp:96 packetization-mode=1; sprop-parameter-sets=Z2QAHqzZQKAv+XARAAADAAEAAAMAMg8WLZY=,aOvjyyLA; profile-level-id=64001E\r\n\
        a=control:streamid=0\r\n\
        m=audio 0 RTP/AVP 97\r\n\
        b=AS:128\r\n\
        a=rtpmap:97 MPEG4-GENERIC/48000/2\r\n\
        a=fmtp:97 profile-level-id=1;mode=AAC-hbr;sizelength=13;indexlength=3;indexdeltalength=3; config=119056E500\r\n\
        a=control:streamid=1\r\n";

        parser2.unmarshal(data2);

        println!(" parser: {:?}", parser2);
    }
}
