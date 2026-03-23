use std::marker::PhantomData;

// use super::events::NetworkEvent;
use async_trait::async_trait;
use borsh::{BorshDeserialize, BorshSerialize};
use futures::prelude::*;
use libp2p::request_response;
use tokio::io;
use wire_message::{WireMessage, wire_message};

#[derive(Clone)]
pub struct PolyProtocol<NetworkEvent: Clone + Send + BorshSerialize + BorshDeserialize + 'static>(
    pub PhantomData<NetworkEvent>,
);

impl<NetworkEvent> request_response::ProtocolName for PolyProtocol<NetworkEvent>
where
    NetworkEvent: Clone + Send + BorshSerialize + BorshDeserialize + 'static,
{
    fn protocol_name(&self) -> &[u8] {
        b"/polybase/0.1.0"
    }
}

#[derive(Debug, Clone, PartialEq)]
#[wire_message]
pub enum Request<T> {
    V1(T),
}

impl<T> WireMessage for Request<T>
where
    T: Clone + BorshSerialize + BorshDeserialize + Send + Sync + 'static,
{
    type Ctx = ();
    type Err = core::convert::Infallible;

    fn version(&self) -> u64 {
        match self {
            Self::V1(_) => 1,
        }
    }

    fn upgrade_once(self, _ctx: &mut Self::Ctx) -> Result<Self, wire_message::Error> {
        match self {
            Self::V1(_) => Err(Self::max_version_error()),
        }
    }
}

#[derive(Debug)]
#[wire_message]
pub enum Response {
    V1,
}

impl WireMessage for Response {
    type Ctx = ();
    type Err = core::convert::Infallible;

    fn version(&self) -> u64 {
        match self {
            Self::V1 => 1,
        }
    }

    fn upgrade_once(self, _ctx: &mut Self::Ctx) -> Result<Self, wire_message::Error> {
        match self {
            Self::V1 => Err(Self::max_version_error()),
        }
    }
}

#[async_trait]
impl<NetworkEvent> request_response::Codec for PolyProtocol<NetworkEvent>
where
    NetworkEvent: Clone + Send + Sync + BorshSerialize + BorshDeserialize + 'static,
{
    type Protocol = PolyProtocol<NetworkEvent>;
    type Request = Request<NetworkEvent>;
    type Response = Response;

    async fn read_request<T>(
        &mut self,
        _: &PolyProtocol<NetworkEvent>,
        io: &mut T,
    ) -> io::Result<Self::Request>
    where
        T: AsyncRead + Unpin + Send,
    {
        let mut buf = Vec::new();
        io.read_to_end(&mut buf).await?;
        let request =
            Request::from_bytes(&buf).map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))?;
        Ok(request)
    }

    async fn read_response<T>(
        &mut self,
        _: &Self::Protocol,
        io: &mut T,
    ) -> io::Result<Self::Response>
    where
        T: AsyncRead + Unpin + Send,
    {
        let mut buf = Vec::new();
        io.read_to_end(&mut buf).await?;
        let response = Response::from_bytes(&buf)
            .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))?;
        Ok(response)
    }

    async fn write_request<T>(
        &mut self,
        _: &Self::Protocol,
        io: &mut T,
        request: Self::Request,
    ) -> io::Result<()>
    where
        T: AsyncWrite + Unpin + Send,
    {
        let data = request
            .to_bytes()
            .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))?;

        io.write_all(&data).await
    }

    async fn write_response<T>(
        &mut self,
        _: &Self::Protocol,
        io: &mut T,
        response: Self::Response,
    ) -> io::Result<()>
    where
        T: AsyncWrite + Unpin + Send,
    {
        let data = response
            .to_bytes()
            .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))?;
        io.write_all(&data).await
    }
}
