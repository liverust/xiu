use {
    crate::{
        amf0::errors::Amf0WriteError, cache::errors::MetadataError, session::errors::SessionError,
    },
    failure::{Backtrace, Fail},
    std::fmt,
    tokio::sync::broadcast::error::RecvError,
};

pub struct RtmpRemuxerError {
    pub value: RtmpRemuxerErrorValue,
}

#[derive(Debug, Fail)]
pub enum RtmpRemuxerErrorValue {
    #[fail(display = "hls error")]
    Error,
    #[fail(display = "session error:{}\n", _0)]
    SessionError(#[cause] SessionError),
    #[fail(display = "amf write error:{}\n", _0)]
    Amf0WriteError(#[cause] Amf0WriteError),
    #[fail(display = "metadata error:{}\n", _0)]
    MetadataError(#[cause] MetadataError),
    #[fail(display = "receive error:{}\n", _0)]
    RecvError(#[cause] RecvError),
    #[fail(display = "stream hub event send error\n")]
    StreamHubEventSendErr,
}
impl From<RecvError> for RtmpRemuxerError {
    fn from(error: RecvError) -> Self {
        RtmpRemuxerError {
            value: RtmpRemuxerErrorValue::RecvError(error),
        }
    }
}

impl From<SessionError> for RtmpRemuxerError {
    fn from(error: SessionError) -> Self {
        RtmpRemuxerError {
            value: RtmpRemuxerErrorValue::SessionError(error),
        }
    }
}

impl From<Amf0WriteError> for RtmpRemuxerError {
    fn from(error: Amf0WriteError) -> Self {
        RtmpRemuxerError {
            value: RtmpRemuxerErrorValue::Amf0WriteError(error),
        }
    }
}

impl From<MetadataError> for RtmpRemuxerError {
    fn from(error: MetadataError) -> Self {
        RtmpRemuxerError {
            value: RtmpRemuxerErrorValue::MetadataError(error),
        }
    }
}

impl fmt::Display for RtmpRemuxerError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        fmt::Display::fmt(&self.value, f)
    }
}
