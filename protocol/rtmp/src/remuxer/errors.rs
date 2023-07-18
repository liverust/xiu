use {
    crate::{
        amf0::errors::Amf0WriteError, cache::errors::MetadataError, session::errors::SessionError,
    },
    failure::{Backtrace, Fail},
    std::fmt,
    tokio::sync::broadcast::error::RecvError,
};

pub struct RemuxerError {
    pub value: RemuxerErrorValue,
}

#[derive(Debug, Fail)]
pub enum RemuxerErrorValue {
    #[fail(display = "hls error")]
    Error,
    #[fail(display = "session error:{}\n", _0)]
    SessionError(#[cause] SessionError),
    #[fail(display = "amf write error:{}\n", _0)]
    Amf0WriteError(#[cause] Amf0WriteError),
    #[fail(display = "metadata error:{}\n", _0)]
    MetadataError(#[cause] MetadataError),
    // #[fail(display = "flv demuxer error:{}\n", _0)]
    // FlvDemuxerError(#[cause] FlvDemuxerError),
    // #[fail(display = "media error:{}\n", _0)]
    // MediaError(#[cause] MediaError),
    #[fail(display = "receive error:{}\n", _0)]
    RecvError(#[cause] RecvError),
}
impl From<RecvError> for RemuxerError {
    fn from(error: RecvError) -> Self {
        RemuxerError {
            value: RemuxerErrorValue::RecvError(error),
        }
    }
}

// impl From<MediaError> for RemuxerError {
//     fn from(error: MediaError) -> Self {
//         RemuxerError {
//             value: RemuxerErrorValue::MediaError(error),
//         }
//     }
// }

impl From<SessionError> for RemuxerError {
    fn from(error: SessionError) -> Self {
        RemuxerError {
            value: RemuxerErrorValue::SessionError(error),
        }
    }
}

// impl From<FlvDemuxerError> for RemuxerError {
//     fn from(error: FlvDemuxerError) -> Self {
//         RemuxerError {
//             value: RemuxerErrorValue::FlvDemuxerError(error),
//         }
//     }
// }

impl From<Amf0WriteError> for RemuxerError {
    fn from(error: Amf0WriteError) -> Self {
        RemuxerError {
            value: RemuxerErrorValue::Amf0WriteError(error),
        }
    }
}

impl From<MetadataError> for RemuxerError {
    fn from(error: MetadataError) -> Self {
        RemuxerError {
            value: RemuxerErrorValue::MetadataError(error),
        }
    }
}

impl fmt::Display for RemuxerError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        fmt::Display::fmt(&self.value, f)
    }
}
