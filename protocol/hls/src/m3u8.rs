use crate::record::Record;

use {
    super::{errors::MediaError, ts::Ts},
    bytes::BytesMut,
    std::{collections::VecDeque, fs, fs::File, io::Write},
};

pub struct Segment {
    /*ts duration*/
    pub duration: i64,
    pub discontinuity: bool,
    /*ts name*/
    pub name: String,
    path: String,
    pub is_eof: bool,
}

impl Segment {
    pub fn new(
        duration: i64,
        discontinuity: bool,
        name: String,
        path: String,
        is_eof: bool,
    ) -> Self {
        Self {
            duration,
            discontinuity,
            name,
            path,
            is_eof,
        }
    }
}

pub struct M3u8 {
    version: u16,
    sequence_no: u64,
    /*What duration should media files be?
    A duration of 10 seconds of media per file seems to strike a reasonable balance for most broadcast content.
    http://devimages.apple.com/iphone/samples/bipbop/bipbopall.m3u8*/
    duration: i64,
    /*How many files should be listed in the index file during a continuous, ongoing session?
    The normal recommendation is 3, but the optimum number may be larger.*/
    live_ts_count: usize,

    segments: VecDeque<Segment>,

    m3u8_folder: String,
    m3u8_name: String,

    ts_handler: Ts,

    record: Option<Record>,
}

impl M3u8 {
    pub fn new(
        duration: i64,
        live_ts_count: usize,
        name: String,
        app_name: String,
        stream_name: String,
        record_path: Option<String>,
    ) -> Self {
        let m3u8_folder = format!("./{app_name}/{stream_name}");
        fs::create_dir_all(m3u8_folder.clone()).unwrap();

        let (ts_record_path, record) = if let Some(path) = record_path {
            /*generate a record folder for one live stream.*/
            let stream_path = format!("{path}/{app_name}_{stream_name}");
            fs::create_dir_all(stream_path.clone()).unwrap();

            let m3u8_path = format!("{stream_path}/{stream_name}.m3u8");
            (Some(stream_path), Some(Record::new(m3u8_path, duration)))
        } else {
            (None, None)
        };

        Self {
            version: 3,
            sequence_no: 0,
            duration,
            live_ts_count,
            segments: VecDeque::new(),
            m3u8_folder,
            m3u8_name: name,
            ts_handler: Ts::new(app_name, stream_name, ts_record_path),
            record,
        }
    }

    pub fn add_segment(
        &mut self,
        duration: i64,
        discontinuity: bool,
        is_eof: bool,
        ts_data: BytesMut,
    ) -> Result<(), MediaError> {
        let segment_count = self.segments.len();

        if segment_count >= self.live_ts_count {
            let segment = self.segments.pop_front().unwrap();
            self.ts_handler.delete(segment.path);
            self.sequence_no += 1;
        }

        self.duration = std::cmp::max(duration, self.duration);

        let (ts_name, ts_path) = self.ts_handler.write(ts_data)?;
        let segment = Segment::new(duration, discontinuity, ts_name, ts_path, is_eof);

        //update record m3u8 content
        if let Some(record) = &mut self.record {
            record.update_m3u8(&segment);
        }
        self.segments.push_back(segment);

        Ok(())
    }

    pub fn clear(&mut self) -> Result<(), MediaError> {
        if let Some(record) = &mut self.record {
            record.flush()?;
        }
        //clear ts
        for segment in &self.segments {
            self.ts_handler.delete(segment.path.clone());
        }
        //clear m3u8
        let m3u8_path = format!("{}/{}", self.m3u8_folder, self.m3u8_name);
        fs::remove_file(m3u8_path)?;

        Ok(())
    }

    pub fn generate_m3u8_header(&self) -> Result<String, MediaError> {
        let mut m3u8_header = "#EXTM3U\n".to_string();
        m3u8_header += format!("#EXT-X-VERSION:{}\n", self.version).as_str();
        m3u8_header += format!("#EXT-X-TARGETDURATION:{}\n", (self.duration + 999) / 1000).as_str();
        m3u8_header += format!("#EXT-X-MEDIA-SEQUENCE:{}\n", self.sequence_no).as_str();

        Ok(m3u8_header)
    }

    pub fn refresh_playlist(&mut self) -> Result<String, MediaError> {
        let mut m3u8_content = self.generate_m3u8_header()?;

        for segment in &self.segments {
            if segment.discontinuity {
                m3u8_content += "#EXT-X-DISCONTINUITY\n";
            }
            m3u8_content += format!(
                "#EXTINF:{:.3}\n{}\n",
                segment.duration as f64 / 1000.0,
                segment.name
            )
            .as_str();

            if segment.is_eof {
                m3u8_content += "#EXT-X-ENDLIST\n";
                break;
            }
        }

        let m3u8_path = format!("{}/{}", self.m3u8_folder, self.m3u8_name);

        let mut file_handler = File::create(m3u8_path).unwrap();
        file_handler.write_all(m3u8_content.as_bytes())?;

        Ok(m3u8_content)
    }
}
