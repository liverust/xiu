use crate::session::define::SessionType;

use super::errors::{RtmpRemuxerError, RtmpRemuxerErrorValue};

use {
    crate::session::common::Common,
    std::time::Duration,
    streamhub::{
        define::{
            FrameData, FrameDataReceiver, NotifyInfo, StreamHubEvent, StreamHubEventSender,
            SubscribeType, SubscriberInfo,
        },
        stream::StreamIdentifier,
        utils::{RandomDigitCount, Uuid},
    },
    tokio::{sync::mpsc, time::sleep},
};
pub struct Rtsp2RtmpRemuxerSession {
    event_producer: StreamHubEventSender,
    //RTMP
    app_name: String,
    stream_name: String,
    rtmp_handler: Common,
    publishe_id: Uuid,
    //RTSP
    data_receiver: FrameDataReceiver,
    stream_path: String,
    subscribe_id: Uuid,
}

impl Rtsp2RtmpRemuxerSession {
    pub fn new(stream_path: String, event_producer: StreamHubEventSender, duration: i64) -> Self {
        let (_, data_consumer) = mpsc::unbounded_channel();
        let eles: Vec<&str> = stream_path.splitn(2, '/').collect();
        let (app_name, stream_name) = if eles.len() < 2 {
            log::warn!(
                "publish_rtmp: the rtsp path only contains stream name: {}",
                stream_path
            );
            (String::from("rtsp"), String::from(eles[0]))
        } else {
            (String::from(eles[0]), String::from(eles[1]))
        };
        Self {
            stream_path,
            app_name,
            stream_name,
            data_receiver: data_consumer,
            event_producer: event_producer.clone(),
            rtmp_handler: Common::new(None, event_producer, SessionType::Server, None),
            subscribe_id: Uuid::new(RandomDigitCount::Four),
            publishe_id: Uuid::new(RandomDigitCount::Four),
        }
    }

    pub async fn run(&mut self) -> Result<(), RtmpRemuxerError> {
        self.subscribe_rtsp().await?;
        self.receive_rtsp_data().await?;

        Ok(())
    }

    pub async fn publish_rtmp(&mut self) -> Result<(), RtmpRemuxerError> {
        self.rtmp_handler
            .publish_to_channels(
                self.app_name.clone(),
                self.stream_name.clone(),
                self.publishe_id,
                0,
            )
            .await?;
        Ok(())
    }

    pub async fn unpublish_rtmp(&mut self) -> Result<(), RtmpRemuxerError> {
        self.rtmp_handler
            .unpublish_to_channels(
                self.app_name.clone(),
                self.stream_name.clone(),
                self.publishe_id,
            )
            .await?;
        Ok(())
    }

    pub async fn receive_rtsp_data(&mut self) -> Result<(), RtmpRemuxerError> {
        let mut retry_count = 0;

        loop {
            if let Some(data) = self.data_receiver.recv().await {
                match data {
                    FrameData::Audio { timestamp, data } => {}
                    FrameData::Video { timestamp, data } => {}
                    _ => continue,
                };
                retry_count = 0;
            } else {
                sleep(Duration::from_millis(100)).await;
                retry_count += 1;
            }
            //When rtmp stream is interupted here we retry 10 times.
            //maybe have a better way to judge the stream status.
            //will do an optimization in the future.
            //todo
            if retry_count > 10 {
                break;
            }
        }

        self.unsubscribe_rtsp().await
    }

    pub async fn subscribe_rtsp(&mut self) -> Result<(), RtmpRemuxerError> {
        let (sender, receiver) = mpsc::unbounded_channel();

        let sub_info = SubscriberInfo {
            id: self.subscribe_id,
            sub_type: SubscribeType::PlayerRtsp,
            notify_info: NotifyInfo {
                request_url: String::from(""),
                remote_addr: String::from(""),
            },
        };

        let subscribe_event = StreamHubEvent::Subscribe {
            identifier: StreamIdentifier::Rtsp {
                stream_path: self.stream_path.clone(),
            },
            info: sub_info,
            sender,
        };

        if self.event_producer.send(subscribe_event).is_err() {
            return Err(RtmpRemuxerError {
                value: RtmpRemuxerErrorValue::StreamHubEventSendErr,
            });
        }

        self.data_receiver = receiver;
        Ok(())
    }

    pub async fn unsubscribe_rtsp(&mut self) -> Result<(), RtmpRemuxerError> {
        let sub_info = SubscriberInfo {
            id: self.subscribe_id,
            sub_type: SubscribeType::PlayerRtsp,
            notify_info: NotifyInfo {
                request_url: String::from(""),
                remote_addr: String::from(""),
            },
        };

        let subscribe_event = StreamHubEvent::UnSubscribe {
            identifier: StreamIdentifier::Rtsp {
                stream_path: self.stream_path.clone(),
            },
            info: sub_info,
        };
        if let Err(err) = self.event_producer.send(subscribe_event) {
            log::error!("unsubscribe_from_channels err {}\n", err);
        }

        Ok(())
    }
}
