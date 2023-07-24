use {
    crate::rtp::errors::{PackerError, UnPackerError},
    bytesio::bytes_errors::BytesReadError,
    bytesio::{bytes_errors::BytesWriteError, bytesio_errors::BytesIOError},
    failure::{Backtrace, Fail},
    std::fmt,
    std::str::Utf8Error,
};

#[derive(Debug)]
pub struct SessionError {
    pub value: SessionErrorValue,
}

#[derive(Debug, Fail)]
pub enum SessionErrorValue {
    #[fail(display = "net io error: {}\n", _0)]
    BytesIOError(#[cause] BytesIOError),
    #[fail(display = "bytes read error: {}\n", _0)]
    BytesReadError(#[cause] BytesReadError),
    #[fail(display = "bytes write error: {}\n", _0)]
    BytesWriteError(#[cause] BytesWriteError),
    #[fail(display = "Utf8Error: {}\n", _0)]
    Utf8Error(#[cause] Utf8Error),
    #[fail(display = "UnPackerError: {}\n", _0)]
    UnPackerError(#[cause] UnPackerError),
    #[fail(display = "stream hub event send error\n")]
    StreamHubEventSendErr,
    #[fail(display = "cannot receive frame data from stream hub\n")]
    CannotReceiveFrameData,
    #[fail(display = "pack error: {}\n", _0)]
    PackerError(#[cause] PackerError),
}

impl From<BytesIOError> for SessionError {
    fn from(error: BytesIOError) -> Self {
        SessionError {
            value: SessionErrorValue::BytesIOError(error),
        }
    }
}

impl From<BytesReadError> for SessionError {
    fn from(error: BytesReadError) -> Self {
        SessionError {
            value: SessionErrorValue::BytesReadError(error),
        }
    }
}

impl From<BytesWriteError> for SessionError {
    fn from(error: BytesWriteError) -> Self {
        SessionError {
            value: SessionErrorValue::BytesWriteError(error),
        }
    }
}

impl From<Utf8Error> for SessionError {
    fn from(error: Utf8Error) -> Self {
        SessionError {
            value: SessionErrorValue::Utf8Error(error),
        }
    }
}

impl From<PackerError> for SessionError {
    fn from(error: PackerError) -> Self {
        SessionError {
            value: SessionErrorValue::PackerError(error),
        }
    }
}

impl From<UnPackerError> for SessionError {
    fn from(error: UnPackerError) -> Self {
        SessionError {
            value: SessionErrorValue::UnPackerError(error),
        }
    }
}

impl fmt::Display for SessionError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        fmt::Display::fmt(&self.value, f)
    }
}

impl Fail for SessionError {
    fn cause(&self) -> Option<&dyn Fail> {
        self.value.cause()
    }

    fn backtrace(&self) -> Option<&Backtrace> {
        self.value.backtrace()
    }
}
