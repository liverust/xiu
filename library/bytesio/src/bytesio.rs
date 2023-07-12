use super::bytesio_errors::{BytesIOError, BytesIOErrorValue};

use bytes::BufMut;
use bytes::Bytes;
use bytes::BytesMut;
use futures::StreamExt;
use tokio::io::AsyncWriteExt;

use std::time::Duration;

use tokio::net::TcpStream;
// use tokio::net::U
use tokio::time::sleep;

use futures::SinkExt;
use std::time::{SystemTime, UNIX_EPOCH};
use tokio_util::codec::BytesCodec;
use tokio_util::codec::Framed;

use async_trait::async_trait;
use std::net::SocketAddr;
use tokio::net::UdpSocket;

#[async_trait]
pub trait TNetIO: Send + Sync {
    async fn write(&mut self, bytes: Bytes) -> Result<(), BytesIOError>;
    async fn read(&mut self) -> Result<BytesMut, BytesIOError>;
    async fn read_timeout(&mut self, duration: Duration) -> Result<BytesMut, BytesIOError>;
}

pub struct UdpIO {
    socket: UdpSocket,
}

impl UdpIO {
    pub async fn new(domain: String, port: u16) -> Option<Self> {
        let address = format!("{}:{}", domain, port);

        if let Ok(socket_addr) = address.parse::<SocketAddr>() {
            if let Ok(socket) = UdpSocket::bind(&socket_addr).await {
                return Some(Self { socket });
            }
        }
        None
    }
}

#[async_trait]
impl TNetIO for UdpIO {
    async fn write(&mut self, bytes: Bytes) -> Result<(), BytesIOError> {
        self.socket.send(bytes.as_ref()).await?;
        Ok(())
    }

    async fn read_timeout(&mut self, duration: Duration) -> Result<BytesMut, BytesIOError> {
        let begin_millseconds = SystemTime::now().duration_since(UNIX_EPOCH).unwrap();

        loop {
            match self.read().await {
                Ok(data) => {
                    return Ok(data);
                }
                Err(_) => {
                    sleep(Duration::from_millis(50)).await;
                    let current_millseconds = SystemTime::now().duration_since(UNIX_EPOCH).unwrap();

                    if current_millseconds - begin_millseconds > duration {
                        return Err(BytesIOError {
                            value: BytesIOErrorValue::TimeoutError,
                        });
                    }
                }
            }
        }
    }

    async fn read(&mut self) -> Result<BytesMut, BytesIOError> {
        let mut buf = vec![0; 4096];
        let len = self.socket.recv(&mut buf).await?;

        let mut rv = BytesMut::new();
        rv.put(&buf[..len]);

        Ok(rv)
    }
}

pub struct TcpIO {
    stream: Framed<TcpStream, BytesCodec>,
    //timeout: Duration,
}

impl TcpIO {
    pub fn new(stream: TcpStream) -> Self {
        Self {
            stream: Framed::new(stream, BytesCodec::new()),
            // timeout: ms,
        }
    }
}

#[async_trait]
impl TNetIO for TcpIO {
    async fn write(&mut self, bytes: Bytes) -> Result<(), BytesIOError> {
        self.stream.send(bytes).await?;

        Ok(())
    }

    async fn read_timeout(&mut self, duration: Duration) -> Result<BytesMut, BytesIOError> {
        let begin_millseconds = SystemTime::now().duration_since(UNIX_EPOCH).unwrap();

        loop {
            match self.read().await {
                Ok(data) => {
                    return Ok(data);
                }
                Err(_) => {
                    sleep(Duration::from_millis(50)).await;
                    let current_millseconds = SystemTime::now().duration_since(UNIX_EPOCH).unwrap();

                    if current_millseconds - begin_millseconds > duration {
                        return Err(BytesIOError {
                            value: BytesIOErrorValue::TimeoutError,
                        });
                    }
                }
            }
        }
    }

    async fn read(&mut self) -> Result<BytesMut, BytesIOError> {
        let message = self.stream.next().await;

        match message {
            Some(data) => match data {
                Ok(bytes) => Ok(bytes),
                Err(err) => Err(BytesIOError {
                    value: BytesIOErrorValue::IOError(err),
                }),
            },
            None => Err(BytesIOError {
                value: BytesIOErrorValue::NoneReturn,
            }),
        }
    }
}

// pub struct BytesIO {
//     stream: Framed<TcpStream, BytesCodec>,
//     //timeout: Duration,
// }

// impl BytesIO {
//     pub fn new(stream: TcpStream) -> Self {
//         Self {
//             stream: Framed::new(stream, BytesCodec::new()),
//             // timeout: ms,
//         }
//     }

//     pub async fn write(&mut self, bytes: Bytes) -> Result<(), BytesIOError> {
//         self.stream.send(bytes).await?;

//         Ok(())
//     }

//     pub async fn read_timeout(&mut self, duration: Duration) -> Result<BytesMut, BytesIOError> {
//         let begin_millseconds = SystemTime::now().duration_since(UNIX_EPOCH).unwrap();

//         loop {
//             match self.read().await {
//                 Ok(data) => {
//                     return Ok(data);
//                 }
//                 Err(_) => {
//                     sleep(Duration::from_millis(50)).await;
//                     let current_millseconds = SystemTime::now().duration_since(UNIX_EPOCH).unwrap();

//                     if current_millseconds - begin_millseconds > duration {
//                         return Err(BytesIOError {
//                             value: BytesIOErrorValue::TimeoutError,
//                         });
//                     }
//                 }
//             }
//         }
//     }

//     pub async fn read(&mut self) -> Result<BytesMut, BytesIOError> {
//         let message = self.stream.next().await;

//         match message {
//             Some(data) => match data {
//                 Ok(bytes) => Ok(bytes),
//                 Err(err) => Err(BytesIOError {
//                     value: BytesIOErrorValue::IOError(err),
//                 }),
//             },
//             None => Err(BytesIOError {
//                 value: BytesIOErrorValue::NoneReturn,
//             }),
//         }
//     }
// }
