use std::fs::File;
use std::io::Write;

use crate::{errors::MediaError, m3u8::Segment};

#[derive(Default)]
pub struct Record {
    pub m3u8_content: String,
    pub m3u8_path: String,
}

impl Record {
    pub fn new(m3u8_path: String, ts_duration: i64) -> Self {
        let mut m3u8_header = "#EXTM3U\n".to_string();
        m3u8_header += "#EXT-X-VERSION:3\n";
        m3u8_header += format!("#EXT-X-TARGETDURATION:{}\n", (ts_duration + 999) / 1000).as_str();
        m3u8_header += "#EXT-X-MEDIA-SEQUENCE:0\n";
        m3u8_header += "#EXT-X-PLAYLIST-TYPE:VOD\n";
        m3u8_header += "#EXT-X-ALLOW-CACHE:YES\n";

        Self {
            m3u8_path,
            m3u8_content: m3u8_header,
        }
    }
    pub fn update_m3u8(&mut self, segment: &Segment) {
        if segment.discontinuity {
            self.m3u8_content += "#EXT-X-DISCONTINUITY\n";
        }
        self.m3u8_content += format!(
            "#EXTINF:{:.3}\n{}\n",
            segment.duration as f64 / 1000.0,
            segment.name
        )
        .as_str();
    }

    pub fn flush(&mut self) -> Result<(), MediaError> {
        self.m3u8_content += "#EXT-X-ENDLIST\n";

        let mut file_handler = File::create(&self.m3u8_path).unwrap();
        file_handler.write_all(self.m3u8_content.as_bytes())?;
        Ok(())
    }
}
