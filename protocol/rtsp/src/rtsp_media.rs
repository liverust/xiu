use rtp::codecs::h264;
use rtp::packetizer::new_packetizer;
use rtp::packetizer::Depacketizer as RTPDepacketizer;
use rtp::packetizer::Packetizer as RTPPacketizer;
use rtp::sequence::new_random_sequencer;

pub struct RtspMedia {
    packetizer: Box<dyn RTPPacketizer + Send + Sync>,
    depacketizer: Box<dyn RTPDepacketizer + Send + Sync>,
}

impl RtspMedia {
    pub fn new(mtu: usize, payload_type: u8, ssrc: u32, clock_rate: u32) -> Self {
        let h264_payloader = Box::new(h264::H264Payloader::default());
        let sequencer = Box::new(new_random_sequencer());

        let pack = new_packetizer(
            mtu,
            payload_type,
            ssrc,
            h264_payloader,
            sequencer,
            clock_rate,
        );

        Self {
            packetizer: Box::new(pack),
            depacketizer: Box::new(h264::H264Packet::default()),
        }
    }
}
