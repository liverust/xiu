use bytes::BytesMut;

pub mod errors;
pub mod rtcp_app;
pub mod rtcp_bye;
pub mod rtcp_header;
pub mod rtcp_rr;

pub trait Unmarshal<T1, T2> {
    fn unmarshal(data: T1) -> Result<Self, T2>
    where
        Self: Sized;
}

pub trait Marshal<T> {
    fn marshal(&self) -> Result<BytesMut, T>;
}
