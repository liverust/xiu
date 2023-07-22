pub mod errors;
pub mod rtsp2rtmp;

use streamhub::{
    define::{BroadcastEvent, BroadcastEventReceiver, StreamHubEventSender},
    stream::StreamIdentifier,
};

use self::{errors::RtmpRemuxerError, rtsp2rtmp::Rtsp2RtmpRemuxerSession};

//Receive publish event from stream hub and
//remux from other protocols to rtmp
pub struct RtmpRemuxer {
    receiver: BroadcastEventReceiver,
    event_producer: StreamHubEventSender,
}

impl RtmpRemuxer {
    pub fn new(receiver: BroadcastEventReceiver, event_producer: StreamHubEventSender) -> Self {
        Self {
            receiver,
            event_producer,
        }
    }
    pub async fn run(&mut self) -> Result<(), RtmpRemuxerError> {
        log::info!("rtmp remuxer start...");

        loop {
            let val = self.receiver.recv().await?;
            log::info!("{:?}", val);
            match val {
                BroadcastEvent::Publish { identifier } => {
                    if let StreamIdentifier::Rtsp { stream_path } = identifier {
                        let mut session =
                            Rtsp2RtmpRemuxerSession::new(stream_path, self.event_producer.clone());
                        tokio::spawn(async move {
                            if let Err(err) = session.run().await {
                                log::error!("rtsp2rtmp session error: {}\n", err);
                            }
                        });
                    }
                }
                _ => {
                    log::trace!("other infos...");
                }
            }
        }
    }
}
