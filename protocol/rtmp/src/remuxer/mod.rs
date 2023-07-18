pub mod errors;
pub mod session;

use streamhub::{
    define::{BroadcastEvent, BroadcastEventReceiver, StreamHubEventSender},
    stream::StreamIdentifier,
};

use self::errors::RemuxerError;

pub struct Rtsp2RtmpRemuxer {
    receiver: BroadcastEventReceiver,
    event_producer: StreamHubEventSender,
}

impl Rtsp2RtmpRemuxer {
    pub async fn run(&mut self) -> Result<(), RemuxerError> {
        loop {
            let val = self.receiver.recv().await?;
            match val {
                BroadcastEvent::Publish { identifier } => {
                    if let StreamIdentifier::Rtsp { stream_path } = identifier {}
                    // if let StreamIdentifier::Rtmp {
                    //     app_name,
                    //     stream_name,
                    // } = identifier
                    // {}
                }
                _ => {
                    log::trace!("other infos...");
                }
            }
        }
    }
}
