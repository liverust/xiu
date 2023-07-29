use {
    super::errors::MediaError,
    bytes::BytesMut,
    std::{fs, fs::File, io::Write},
};

pub struct Ts {
    ts_number: u32,
    live_path: String,
    record_path: Option<String>,
}

impl Ts {
    pub fn new(app_name: String, stream_name: String, record_path: Option<String>) -> Self {
        let live_path = format!("./{app_name}/{stream_name}");
        fs::create_dir_all(live_path.clone()).unwrap();

        Self {
            ts_number: 0,
            live_path,
            record_path,
        }
    }
    pub fn write(&mut self, data: BytesMut) -> Result<(String, String), MediaError> {
        let ts_file_name = format!("{}.ts", self.ts_number);
        let ts_file_path = format!("{}/{}", self.live_path, ts_file_name);
        self.ts_number += 1;

        let mut ts_file_handler = File::create(ts_file_path.clone())?;
        ts_file_handler.write_all(&data[..])?;

        /*record the ts file to specified path configured in configuration file */
        if let Some(path) = &self.record_path {
            let record_file_path = format!("{path}/{ts_file_name}");
            if let Err(err) = fs::copy(&ts_file_path, record_file_path) {
                log::error!("reord ts file: {ts_file_path} err:{err}");
            }
        }

        Ok((ts_file_name, ts_file_path))
    }
    pub fn delete(&mut self, ts_file_name: String) {
        fs::remove_file(ts_file_name).unwrap();
    }
}
