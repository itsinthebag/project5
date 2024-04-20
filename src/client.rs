use std::net::SocketAddr;

use tokio::codec::{FramedRead, FramedWrite, LengthDelimitedCodec};
use tokio::io::{AsyncRead, ReadHalf, WriteHalf};
use tokio::net::TcpStream;
use tokio::prelude::{Future, Sink, Stream};
use tokio_serde_json::{ReadJson, WriteJson};

use crate::common::{Request, Response};
use crate::KvsError;

pub struct KvsClient {
    read_json: ReadJson<FramedRead<ReadHalf<TcpStream>, LengthDelimitedCodec>, Response>,
    write_json: WriteJson<FramedWrite<WriteHalf<TcpStream>, LengthDelimitedCodec>, Request>,
}

impl KvsClient {
    pub fn connect(addr: SocketAddr) -> impl Future<Item = Self, Error = KvsError> {
        TcpStream::connect(&addr)
            .map(|tcp| {
                let (read_half, write_half) = tcp.split();
                KvsClient {
                    read_json: ReadJson::new(FramedRead::new(read_half, LengthDelimitedCodec::new())),
                    write_json: WriteJson::new(FramedWrite::new(write_half, LengthDelimitedCodec::new())),
                }
            })
            .map_err(|e| e.into())
    }

    pub fn get(self, key: String) -> impl Future<Item = (Option<String>, Self), Error = KvsError> {
        self.send_request(Request::Get {key})
            .and_then(move |(response, client)| {
                match response {
                    Some(Response::Get(value)) => Ok((value, client)),
                    Some(Response::Err(msg)) => Err(KvsError::StringError(msg)),
                    Some(_) => Err(KvsError::StringError("Invalid response".to_owned())),
                    None => Err(KvsError::StringError("no response".to_owned())),
                }
            })
    }

    pub fn set(self, key: String, value: String) -> impl Future<Item = Self, Error = KvsError> {
        self.send_request(Request::Set {key, value})
            .and_then(move |(response, client)| {
                match response {
                    Some(Response::Set) => Ok(client),
                    Some(Response::Err(msg)) => Err(KvsError::StringError(msg)),
                    Some(_) => Err(KvsError::StringError("Invalid response".to_owned())),
                    None => Err(KvsError::StringError("no response".to_owned())),
                }
            })
    }

    pub fn remove(self, key: String) -> impl Future<Item = Self, Error = KvsError> {
        self.send_request(Request::Remove {key})
            .and_then(move |(response, client)| {
                match response {
                    Some(Response::Remove) => Ok(client),
                    Some(Response::Err(msg)) => Err(KvsError::StringError(msg)),
                    Some(_) => Err(KvsError::StringError("Invalid response".to_owned())),
                    None => Err(KvsError::StringError("no response".to_owned())),
                }
            })
    }

    fn send_request(self, request: Request) -> impl Future<Item = (Option<Response>, Self), Error = KvsError> {
        let read_json = self.read_json;
        self.write_json
            .send(request)
            .and_then(move |write_json| {
                read_json
                    .into_future()
                    .map(move |(response, read_json)| {
                        let client = KvsClient {
                            read_json,
                            write_json,
                        };
                        (response, client)
                    })
                    .map_err(|(err, ..)| err)
            })
            .map_err(|err| err)
    }
}