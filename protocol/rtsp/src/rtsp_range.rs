use super::global_trait::TMsgConverter;
use super::rtsp_utils;

#[derive(Debug, Clone, Default, PartialEq)]
pub enum RtspRangeType {
    #[default]
    NPT,
    CLOCK,
}

#[derive(Debug, Clone, Default)]
pub struct RtspRange {
    range_type: RtspRangeType,
    begin: i64,
    end: Option<i64>,
}

impl TMsgConverter for RtspRange {
    fn unmarshal(raw_data: &str) -> Option<Self> {
        //a=range:clock=20210520T063812Z-20210520T064816Z
        //a=range:npt=now-
        //a=range:npt=0-
        let mut rtsp_range = RtspRange::default();

        let kv: Vec<&str> = raw_data.splitn(2, '=').collect();
        if kv.len() < 2 {
            return None;
        }

        match kv[0] {
            "clock" => {
                rtsp_range.range_type = RtspRangeType::CLOCK;

                let ranges: Vec<&str> = kv[1].split('-').collect();
                rtsp_range.begin = rtsp_utils::time_2_epoch_seconds(ranges[0]);
                if ranges.len() > 1 {
                    rtsp_range.end = Some(rtsp_utils::time_2_epoch_seconds(ranges[1]));
                }
            }
            "npt" => {
                rtsp_range.range_type = RtspRangeType::NPT;
                let ranges: Vec<&str> = kv[1].split('-').collect();

                let get_npt_time = |range_time: &str| -> i64 {
                    if let (Some(hour), Some(minute), Some(second), mill) =
                        rtsp_utils::scanf!(range_time, |c| c == ':' || c == '.', i64, i64, i64, i64)
                    {
                        let mut result = (hour * 3600 + minute * 60 + second) * 1000;
                        if let Some(m) = mill {
                            result += m;
                        }
                        result
                    } else {
                        0
                    }
                };

                match ranges[0] {
                    "now" => {
                        rtsp_range.begin = 0;
                    }
                    _ => {
                        rtsp_range.begin = get_npt_time(ranges[0]);
                    }
                }

                if ranges.len() == 2 {
                    rtsp_range.end = Some(get_npt_time(ranges[1]));
                }
            }
            _ => {
                log::info!("{} not parsed..", kv[0]);
            }
        }

        Some(rtsp_range)
    }

    fn marshal(&self) -> String {
        String::default()
    }
}
