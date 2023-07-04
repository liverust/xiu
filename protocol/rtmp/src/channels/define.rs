use {
    super::errors::ChannelError,
    crate::session::common::{PublisherInfo, SubscriberInfo},
    crate::statistics::StreamStatistics,
    bytes::BytesMut,
    serde::Serialize,
    std::fmt,
    tokio::sync::{broadcast, mpsc, oneshot},
    uuid::Uuid,
};

use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;

use crate::session::define::SubscribeType;

#[derive(Clone)]
pub enum ChannelData {
    Video { timestamp: u32, data: BytesMut },
    Audio { timestamp: u32, data: BytesMut },
    MetaData { timestamp: u32, data: BytesMut },
}

pub type ChannelDataSender = mpsc::UnboundedSender<ChannelData>;
pub type ChannelDataReceiver = mpsc::UnboundedReceiver<ChannelData>;

pub type ChannelEventProducer = mpsc::UnboundedSender<ChannelEvent>;
pub type ChannelEventConsumer = mpsc::UnboundedReceiver<ChannelEvent>;

pub type ClientEventProducer = broadcast::Sender<ClientEvent>;
pub type ClientEventConsumer = broadcast::Receiver<ClientEvent>;

pub type TransmitterEventProducer = mpsc::UnboundedSender<TransmitterEvent>;
pub type TransmitterEventConsumer = mpsc::UnboundedReceiver<TransmitterEvent>;

pub type AvStatisticSender = mpsc::UnboundedSender<StreamStatistics>;
pub type AvStatisticReceiver = mpsc::UnboundedReceiver<StreamStatistics>;

pub type StreamStatisticSizeSender = oneshot::Sender<usize>;
pub type StreamStatisticSizeReceiver = oneshot::Sender<usize>;

type ChannelResponder<T> = oneshot::Sender<T>;

pub trait TStreamHandler: Send + Sync {
    fn send_cache_data(
        &self,
        sender: ChannelDataSender,
        sub_type: SubscribeType,
    ) -> Result<(), ChannelError>;
    fn get_statistic_data(&self) -> StreamStatistics;
}

pub type SendCacheDataFn = Box<
    dyn (Fn(
            ChannelDataSender,
            SubscribeType,
        ) -> Pin<Box<dyn Future<Output = Result<(), ChannelError>> + Send + 'static>>)
        + Send
        + Sync,
>;

#[derive(Serialize)]
pub enum ChannelEvent {
    Subscribe {
        app_name: String,
        stream_name: String,
        info: SubscriberInfo,
        #[serde(skip_serializing)]
        sender: ChannelDataSender,
    },
    UnSubscribe {
        app_name: String,
        stream_name: String,
        info: SubscriberInfo,
    },
    Publish {
        app_name: String,
        stream_name: String,
        info: PublisherInfo,
        #[serde(skip_serializing)]
        receiver: ChannelDataReceiver,
        #[serde(skip_serializing)]
        cache_sender: SendCacheDataFn,
        #[serde(skip_serializing)]
        stream_handler: Box<dyn TStreamHandler>,
    },
    UnPublish {
        app_name: String,
        stream_name: String,
        info: PublisherInfo,
    },
    #[serde(skip_serializing)]
    ApiStatistic {
        data_sender: AvStatisticSender,
        size_sender: StreamStatisticSizeSender,
    },
    #[serde(skip_serializing)]
    ApiKickClient { id: Uuid },
}

#[derive(Debug)]
pub enum TransmitterEvent {
    Subscribe {
        sender: ChannelDataSender,
        info: SubscriberInfo,
    },
    UnSubscribe {
        info: SubscriberInfo,
    },
    UnPublish {},

    Api {
        sender: AvStatisticSender,
    },
}

impl fmt::Display for TransmitterEvent {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{:?}", *self)
    }
}

#[derive(Debug, Clone)]
pub enum ClientEvent {
    /*Need publish(push) a stream to other rtmp server*/
    Publish {
        app_name: String,
        stream_name: String,
    },
    UnPublish {
        app_name: String,
        stream_name: String,
    },
    /*Need subscribe(pull) a stream from other rtmp server*/
    Subscribe {
        app_name: String,
        stream_name: String,
    },
    UnSubscribe {
        app_name: String,
        stream_name: String,
    },
}

//Used for kickoff
#[derive(Debug, Clone)]
pub enum PubSubInfo {
    Subscribe {
        app_name: String,
        stream_name: String,
        sub_info: SubscriberInfo,
    },

    Publish {
        app_name: String,
        stream_name: String,
    },
}
