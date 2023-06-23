use super::rtp::utils::TPacker;
use super::rtp::utils::TUnPacker;
use super::rtsp_transport::RtspTransport;

pub struct RtspTrack<T1: TPacker, T2: TUnPacker> {
    transport: RtspTransport,
    rtp_packer: T1,
    rtp_unpacker: T2,
    ssrc: u32,
}
