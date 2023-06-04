use {
    failure::{Backtrace, Fail},
    std::fmt,
};

use bytesio::bytes_errors::BytesReadError;
use bytesio::bytes_errors::BytesWriteError;

#[derive(Debug)]
pub struct RtpH264PackerError {
    pub value: RtpH264PackerErrorValue,
}
#[derive(Debug, Fail)]
pub enum RtpH264PackerErrorValue {
    #[fail(display = "bytes read error: {}\n", _0)]
    BytesReadError(BytesReadError),
    #[fail(display = "bytes write error: {}\n", _0)]
    BytesWriteError(BytesWriteError),
}

impl From<BytesReadError> for RtpH264PackerError {
    fn from(error: BytesReadError) -> Self {
        RtpH264PackerError {
            value: RtpH264PackerErrorValue::BytesReadError(error),
        }
    }
}

impl From<BytesWriteError> for RtpH264PackerError {
    fn from(error: BytesWriteError) -> Self {
        RtpH264PackerError {
            value: RtpH264PackerErrorValue::BytesWriteError(error),
        }
    }
}
