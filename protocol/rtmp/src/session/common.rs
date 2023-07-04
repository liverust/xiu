use serde::ser::SerializeStruct;

use {
    super::{
        define::{PublishType, SessionType, SubscribeType},
        errors::{SessionError, SessionErrorValue},
    },
    crate::{
        cache::errors::CacheError,
        cache::Cache,
        channels::define::{
            ChannelData, ChannelDataReceiver, ChannelDataSender, ChannelEvent,
            ChannelEventProducer, SendCacheDataFn, TStreamHandler,
        },
        channels::errors::{ChannelError, ChannelErrorValue},
        chunk::{
            define::{chunk_type, csid_type},
            packetizer::ChunkPacketizer,
            ChunkInfo,
        },
        messages::define::msg_type_id,
        statistics::StreamStatistics,
    },
    async_trait::async_trait,
    bytes::BytesMut,
    bytesio::bytesio::BytesIO,
    serde::{Serialize, Serializer},
    std::fmt,
    std::{net::SocketAddr, sync::Arc, time::Duration},
    tokio::{
        sync::{mpsc, oneshot, Mutex},
        time::sleep,
    },
    uuid::Uuid,
};

#[derive(Debug, Serialize, Clone)]
pub struct NotifyInfo {
    pub request_url: String,
    pub remote_addr: String,
}
#[derive(Debug, Clone)]
pub struct SubscriberInfo {
    pub id: Uuid,
    pub sub_type: SubscribeType,
    pub notify_info: NotifyInfo,
}

impl Serialize for SubscriberInfo {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        // 3 is the number of fields in the struct.
        let mut state = serializer.serialize_struct("SubscriberInfo", 3)?;

        state.serialize_field("id", &self.id.to_string())?;
        state.serialize_field("sub_type", &self.sub_type)?;
        state.serialize_field("notify_info", &self.notify_info)?;
        state.end()
    }
}

#[derive(Debug, Clone)]
pub struct PublisherInfo {
    pub id: Uuid,
    pub sub_type: PublishType,
    pub notify_info: NotifyInfo,
}

impl Serialize for PublisherInfo {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        // 3 is the number of fields in the struct.
        let mut state = serializer.serialize_struct("PublisherInfo", 3)?;

        state.serialize_field("id", &self.id.to_string())?;
        state.serialize_field("sub_type", &self.sub_type)?;
        state.serialize_field("notify_info", &self.notify_info)?;
        state.end()
    }
}
pub struct Common {
    packetizer: ChunkPacketizer,

    data_receiver: ChannelDataReceiver,
    data_sender: ChannelDataSender,

    event_producer: ChannelEventProducer,
    pub session_type: SessionType,

    /*save the client side socket connected to the SeverSession */
    remote_addr: Option<SocketAddr>,
    /*request URL from client*/
    pub request_url: String,
    pub stream_handler: Arc<StreamHandler>,
    /*cache is used to save RTMP sequence/gops/meta data
    which needs to be send to client(player) */
    /*The cache will be used in different threads(save
    cache in one thread and send cache data to different clients
    in other threads) */
    // pub cache: Option<Arc<Mutex<Cache>>>,
}

impl Common {
    pub fn new(
        net_io: Arc<Mutex<BytesIO>>,
        event_producer: ChannelEventProducer,
        session_type: SessionType,
        remote_addr: Option<SocketAddr>,
    ) -> Self {
        //only used for init,since I don't found a better way to deal with this.
        let (init_producer, init_consumer) = mpsc::unbounded_channel();

        Self {
            packetizer: ChunkPacketizer::new(Arc::clone(&net_io)),

            data_sender: init_producer,
            data_receiver: init_consumer,

            event_producer,
            session_type,
            remote_addr,
            request_url: String::default(),
            stream_handler: Arc::new(StreamHandler::new()),
            //cache: None,
        }
    }
    pub async fn send_channel_data(&mut self) -> Result<(), SessionError> {
        let mut retry_times = 0;
        loop {
            if let Some(data) = self.data_receiver.recv().await {
                match data {
                    ChannelData::Audio { timestamp, data } => {
                        self.send_audio(data, timestamp).await?;
                    }
                    ChannelData::Video { timestamp, data } => {
                        self.send_video(data, timestamp).await?;
                    }
                    ChannelData::MetaData { timestamp, data } => {
                        self.send_metadata(data, timestamp).await?;
                    }
                }
            } else {
                retry_times += 1;
                log::debug!(
                    "send_channel_data: no data receives ,retry {} times!",
                    retry_times
                );

                if retry_times > 10 {
                    return Err(SessionError {
                        value: SessionErrorValue::NoMediaDataReceived,
                    });
                }
            }
        }
    }

    pub async fn send_audio(&mut self, data: BytesMut, timestamp: u32) -> Result<(), SessionError> {
        let mut chunk_info = ChunkInfo::new(
            csid_type::AUDIO,
            chunk_type::TYPE_0,
            timestamp,
            data.len() as u32,
            msg_type_id::AUDIO,
            0,
            data,
        );

        self.packetizer.write_chunk(&mut chunk_info).await?;

        Ok(())
    }

    pub async fn send_video(&mut self, data: BytesMut, timestamp: u32) -> Result<(), SessionError> {
        let mut chunk_info = ChunkInfo::new(
            csid_type::VIDEO,
            chunk_type::TYPE_0,
            timestamp,
            data.len() as u32,
            msg_type_id::VIDEO,
            0,
            data,
        );

        self.packetizer.write_chunk(&mut chunk_info).await?;

        Ok(())
    }

    pub async fn send_metadata(
        &mut self,
        data: BytesMut,
        timestamp: u32,
    ) -> Result<(), SessionError> {
        let mut chunk_info = ChunkInfo::new(
            csid_type::DATA_AMF0_AMF3,
            chunk_type::TYPE_0,
            timestamp,
            data.len() as u32,
            msg_type_id::DATA_AMF0,
            0,
            data,
        );

        self.packetizer.write_chunk(&mut chunk_info).await?;
        Ok(())
    }

    pub async fn on_video_data(
        &mut self,
        data: &mut BytesMut,
        timestamp: &u32,
    ) -> Result<(), SessionError> {
        let channel_data = ChannelData::Video {
            timestamp: *timestamp,
            data: data.clone(),
        };

        match self.data_sender.send(channel_data) {
            Ok(_) => {}
            Err(err) => {
                log::error!("send video err: {}", err);
                return Err(SessionError {
                    value: SessionErrorValue::SendChannelDataErr,
                });
            }
        }

        self.stream_handler
            .save_video_data(data, *timestamp)
            .await?;

        // if let Some(cache) = &mut self.cache {
        //     cache.lock().await.save_video_data(data, *timestamp).await?;
        // }
        Ok(())
    }

    pub async fn on_audio_data(
        &mut self,
        data: &mut BytesMut,
        timestamp: &u32,
    ) -> Result<(), SessionError> {
        let channel_data = ChannelData::Audio {
            timestamp: *timestamp,
            data: data.clone(),
        };

        match self.data_sender.send(channel_data) {
            Ok(_) => {}
            Err(err) => {
                log::error!("receive audio err {}\n", err);
                return Err(SessionError {
                    value: SessionErrorValue::SendChannelDataErr,
                });
            }
        }

        self.stream_handler
            .save_audio_data(data, *timestamp)
            .await?;

        // if let Some(cache) = &mut self.cache {
        //     cache.lock().await.save_audio_data(data, *timestamp).await?;
        // }

        Ok(())
    }

    pub async fn on_meta_data(
        &mut self,
        data: &mut BytesMut,
        timestamp: &u32,
    ) -> Result<(), SessionError> {
        let channel_data = ChannelData::MetaData {
            timestamp: *timestamp,
            data: data.clone(),
        };

        match self.data_sender.send(channel_data) {
            Ok(_) => {}
            Err(_) => {
                return Err(SessionError {
                    value: SessionErrorValue::SendChannelDataErr,
                })
            }
        }

        self.stream_handler.save_metadata(data, *timestamp).await;

        // if let Some(cache) = &mut self.cache {
        //     cache.lock().await.save_metadata(data, *timestamp);
        // }

        Ok(())
    }

    fn get_subscriber_info(&mut self, sub_id: Uuid) -> SubscriberInfo {
        let remote_addr = if let Some(addr) = self.remote_addr {
            addr.to_string()
        } else {
            String::from("unknown")
        };

        match self.session_type {
            SessionType::Client => SubscriberInfo {
                id: sub_id,
                /*rtmp local client subscribe from local rtmp session
                and publish(relay) the rtmp steam to remote RTMP server*/
                sub_type: SubscribeType::PublisherRtmp,
                notify_info: NotifyInfo {
                    request_url: self.request_url.clone(),
                    remote_addr,
                },
            },
            SessionType::Server => SubscriberInfo {
                id: sub_id,
                /* rtmp player from remote clent */
                sub_type: SubscribeType::PlayerRtmp,
                notify_info: NotifyInfo {
                    request_url: self.request_url.clone(),
                    remote_addr,
                },
            },
        }
    }

    fn get_publisher_info(&mut self, sub_id: Uuid) -> PublisherInfo {
        let remote_addr = if let Some(addr) = self.remote_addr {
            addr.to_string()
        } else {
            String::from("unknown")
        };

        match self.session_type {
            SessionType::Client => PublisherInfo {
                id: sub_id,
                sub_type: PublishType::SubscriberRtmp,
                notify_info: NotifyInfo {
                    request_url: self.request_url.clone(),
                    remote_addr,
                },
            },
            SessionType::Server => PublisherInfo {
                id: sub_id,
                sub_type: PublishType::PushRtmp,
                notify_info: NotifyInfo {
                    request_url: self.request_url.clone(),
                    remote_addr,
                },
            },
        }
    }

    /*Subscribe from local channels and then send data to retmote common player or local RTMP relay push client*/
    pub async fn subscribe_from_channels(
        &mut self,
        app_name: String,
        stream_name: String,
        sub_id: Uuid,
    ) -> Result<(), SessionError> {
        log::info!(
            "subscribe_from_channels, app_name: {} stream_name: {} subscribe_id: {}",
            app_name,
            stream_name.clone(),
            sub_id
        );

        let mut retry_count: u8 = 0;

        loop {
            let (sender, receiver) = mpsc::unbounded_channel();
            let subscribe_event = ChannelEvent::Subscribe {
                app_name: app_name.clone(),
                stream_name: stream_name.clone(),
                info: self.get_subscriber_info(sub_id),
                sender,
            };
            let rv = self.event_producer.send(subscribe_event);

            if rv.is_err() {
                return Err(SessionError {
                    value: SessionErrorValue::ChannelEventSendErr,
                });
            }

            self.data_receiver = receiver;
            break;

            // match receiver.await {
            //     Ok(consumer) => {
            //         self.data_receiver = consumer;
            //         break;
            //     }
            //     Err(_) => {
            //         if retry_count > 10 {
            //             return Err(SessionError {
            //                 value: SessionErrorValue::SubscribeCountLimitReach,
            //             });
            //         }
            //     }
            // }

            sleep(Duration::from_millis(800)).await;
            retry_count += 1;
        }

        Ok(())
    }

    pub async fn unsubscribe_from_channels(
        &mut self,
        app_name: String,
        stream_name: String,
        sub_id: Uuid,
    ) -> Result<(), SessionError> {
        let subscribe_event = ChannelEvent::UnSubscribe {
            app_name,
            stream_name,
            info: self.get_subscriber_info(sub_id),
        };
        if let Err(err) = self.event_producer.send(subscribe_event) {
            log::error!("unsubscribe_from_channels err {}\n", err);
        }

        Ok(())
    }

    /*Begin to receive stream data from remote RTMP push client or local RTMP relay pull client*/
    pub async fn publish_to_channels(
        &mut self,
        app_name: String,
        stream_name: String,
        pub_id: Uuid,
        gop_num: usize,
    ) -> Result<(), SessionError> {
        // let (sender, receiver) = oneshot::channel();

        let cache = Cache::new(app_name.clone(), stream_name.clone(), gop_num);

        self.stream_handler.set_cache(cache).await;
        // let cache_sender: SendCacheDataFn =
        //     Box::new(move |sender: ChannelDataSender, sub_type: SubscribeType| {
        //         let cache_clone_in = cache.clone();
        //         Box::pin(async move {
        //             if let Some(meta_body_data) = cache_clone_in.lock().await.get_metadata() {
        //                 sender.send(meta_body_data).map_err(|_| ChannelError {
        //                     value: ChannelErrorValue::SendError,
        //                 })?;
        //             }
        //             if let Some(audio_seq_data) = cache_clone_in.lock().await.get_audio_seq() {
        //                 sender.send(audio_seq_data).map_err(|_| ChannelError {
        //                     value: ChannelErrorValue::SendError,
        //                 })?;
        //             }
        //             if let Some(video_seq_data) = cache_clone_in.lock().await.get_video_seq() {
        //                 sender.send(video_seq_data).map_err(|_| ChannelError {
        //                     value: ChannelErrorValue::SendError,
        //                 })?;
        //             }
        //             match sub_type {
        //                 SubscribeType::PlayerRtmp
        //                 | SubscribeType::PlayerHttpFlv
        //                 | SubscribeType::PlayerHls
        //                 | SubscribeType::GenerateHls => {
        //                     if let Some(gops_data) = cache_clone_in.lock().await.get_gops_data() {
        //                         for gop in gops_data {
        //                             for channel_data in gop.get_frame_data() {
        //                                 sender.send(channel_data).map_err(|_| ChannelError {
        //                                     value: ChannelErrorValue::SendError,
        //                                 })?;
        //                             }
        //                         }
        //                     }
        //                 }
        //                 SubscribeType::PublisherRtmp => {}
        //             }
        //             Ok(())
        //         })
        //     });

        // let common = Common::new(net_io, event_producer, session_type, remote_addr);

        let (sender, receiver) = mpsc::unbounded_channel();

        let publish_event = ChannelEvent::Publish {
            app_name,
            stream_name,
            receiver,
            info: self.get_publisher_info(pub_id),

            stream_handler: self.stream_handler.clone(),
        };

        let rv = self.event_producer.send(publish_event);
        if rv.is_err() {
            return Err(SessionError {
                value: SessionErrorValue::ChannelEventSendErr,
            });
        }

        self.data_sender = sender;

        Ok(())
    }

    pub async fn unpublish_to_channels(
        &mut self,
        app_name: String,
        stream_name: String,
        pub_id: Uuid,
    ) -> Result<(), SessionError> {
        log::info!(
            "unpublish_to_channels, app_name:{}, stream_name:{}",
            app_name,
            stream_name
        );
        let unpublish_event = ChannelEvent::UnPublish {
            app_name: app_name.clone(),
            stream_name: stream_name.clone(),
            info: self.get_publisher_info(pub_id),
        };

        let rv = self.event_producer.send(unpublish_event);
        match rv {
            Err(_) => {
                log::error!(
                    "unpublish_to_channels error.app_name: {}, stream_name: {}",
                    app_name,
                    stream_name
                );
                return Err(SessionError {
                    value: SessionErrorValue::ChannelEventSendErr,
                });
            }
            _ => {
                log::info!(
                    "unpublish_to_channels successfully.app_name: {}, stream_name: {}",
                    app_name,
                    stream_name
                );
            }
        }
        Ok(())
    }
}

pub struct StreamHandler {
    /*cache is used to save RTMP sequence/gops/meta data
    which needs to be send to client(player) */
    /*The cache will be used in different threads(save
    cache in one thread and send cache data to different clients
    in other threads) */
    pub cache: Mutex<Option<Cache>>,
}

impl StreamHandler {
    pub fn new() -> Self {
        Self {
            cache: Mutex::new(None),
        }
    }

    pub async fn set_cache(&self, cache: Cache) {
        *self.cache.lock().await = Some(cache);
    }

    pub async fn save_video_data(
        &self,
        chunk_body: &BytesMut,
        timestamp: u32,
    ) -> Result<(), CacheError> {
        if let Some(cache) = &mut *self.cache.lock().await {
            cache.save_video_data(chunk_body, timestamp).await?;
        }
        Ok(())
    }

    pub async fn save_audio_data(
        &self,
        chunk_body: &BytesMut,
        timestamp: u32,
    ) -> Result<(), CacheError> {
        if let Some(cache) = &mut *self.cache.lock().await {
            cache.save_audio_data(chunk_body, timestamp).await?;
        }
        Ok(())
    }

    pub async fn save_metadata(&self, chunk_body: &BytesMut, timestamp: u32) {
        if let Some(cache) = &mut *self.cache.lock().await {
            cache.save_metadata(chunk_body, timestamp);
        }
    }
}

#[async_trait]
impl TStreamHandler for StreamHandler {
    async fn send_cache_data(
        &self,
        sender: ChannelDataSender,
        sub_type: SubscribeType,
    ) -> Result<(), ChannelError> {
        if let Some(cache) = &mut *self.cache.lock().await {
            if let Some(meta_body_data) = cache.get_metadata() {
                sender.send(meta_body_data).map_err(|_| ChannelError {
                    value: ChannelErrorValue::SendError,
                })?;
            }
            if let Some(audio_seq_data) = cache.get_audio_seq() {
                sender.send(audio_seq_data).map_err(|_| ChannelError {
                    value: ChannelErrorValue::SendError,
                })?;
            }
            if let Some(video_seq_data) = cache.get_video_seq() {
                sender.send(video_seq_data).map_err(|_| ChannelError {
                    value: ChannelErrorValue::SendError,
                })?;
            }
            match sub_type {
                SubscribeType::PlayerRtmp
                | SubscribeType::PlayerHttpFlv
                | SubscribeType::PlayerHls
                | SubscribeType::GenerateHls => {
                    if let Some(gops_data) = cache.get_gops_data() {
                        for gop in gops_data {
                            for channel_data in gop.get_frame_data() {
                                sender.send(channel_data).map_err(|_| ChannelError {
                                    value: ChannelErrorValue::SendError,
                                })?;
                            }
                        }
                    }
                }
                SubscribeType::PublisherRtmp => {}
            }
        }

        Ok(())
    }
    async fn get_statistic_data(&self) -> StreamStatistics {
        StreamStatistics::default()
    }
}

impl fmt::Debug for Common {
    fn fmt(&self, fmt: &mut std::fmt::Formatter<'_>) -> Result<(), std::fmt::Error> {
        write!(fmt, "S2 {{ member: {:?} }}", self.request_url)
    }
}
