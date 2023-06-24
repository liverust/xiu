use bytes::BytesMut;

pub mod errors;
pub mod rtcp_app;
pub mod rtcp_bye;
pub mod rtcp_context;
pub mod rtcp_header;
pub mod rtcp_rr;

const RTCP_SR: u8 = 200;
const RTCP_RR: u8 = 201;
const RTCP_SDES: u8 = 202;
const RTCP_BYE: u8 = 203;
const RTCP_APP: u8 = 204;
