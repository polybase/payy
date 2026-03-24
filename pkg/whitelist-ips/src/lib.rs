use core::fmt;
use std::{
    collections::{HashSet, VecDeque},
    net::IpAddr,
    task::{Context, Poll, Waker},
};

use libp2p::{
    Multiaddr, PeerId,
    multiaddr::Protocol,
    swarm::{
        CloseConnection, ConnectionDenied, ConnectionId, NetworkBehaviour, PollParameters,
        THandler, THandlerInEvent, ToSwarm, dummy,
    },
};

#[derive(Debug, Clone, Default)]
#[non_exhaustive]
pub struct Config {
    pub whitelisted_ips: HashSet<IpAddr>,
}

#[derive(Debug, Clone, Default)]
pub struct Behaviour {
    config: Config,
    close_connections: VecDeque<PeerId>,
    waker: Option<Waker>,
}

impl Behaviour {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn new_with_config(config: Config) -> Self {
        Self {
            config,
            ..Self::default()
        }
    }

    pub fn whitelisted_ips(&self) -> &HashSet<IpAddr> {
        &self.config.whitelisted_ips
    }

    pub fn whitelisted_ips_mut(&mut self) -> &mut HashSet<IpAddr> {
        &mut self.config.whitelisted_ips
    }
}

impl NetworkBehaviour for Behaviour {
    type ConnectionHandler = libp2p::swarm::dummy::ConnectionHandler;
    type OutEvent = ();

    fn on_swarm_event(&mut self, _event: libp2p::swarm::FromSwarm<Self::ConnectionHandler>) {
        // do nothing
    }

    fn on_connection_handler_event(
        &mut self,
        _peer_id: libp2p::PeerId,
        _connection_id: libp2p::swarm::ConnectionId,
        _event: libp2p::swarm::THandlerOutEvent<Self>,
    ) {
        #[cfg(debug_assertions)]
        {
            unreachable!()
        }

        #[cfg(not(debug_assertions))]
        {
            tracing::warn!(
                "whitelist_ips::Behaviour::on_connection_handler_event called, which should be impossible"
            )
        }
    }

    fn handle_established_inbound_connection(
        &mut self,
        _: ConnectionId,
        peer: PeerId,
        _local_addr: &Multiaddr,
        remote_addr: &Multiaddr,
    ) -> Result<THandler<Self>, ConnectionDenied> {
        if !allowed(remote_addr, &self.config.whitelisted_ips) {
            return Err(ConnectionDenied::new(NotAllowed {
                peer,
                addr: remote_addr.clone(),
            }));
        }

        Ok(dummy::ConnectionHandler)
    }

    fn poll(
        &mut self,
        cx: &mut Context<'_>,
        _: &mut impl PollParameters,
    ) -> Poll<ToSwarm<Self::OutEvent, THandlerInEvent<Self>>> {
        if let Some(peer) = self.close_connections.pop_front() {
            return Poll::Ready(ToSwarm::CloseConnection {
                peer_id: peer,
                connection: CloseConnection::All,
            });
        }

        self.waker = Some(cx.waker().clone());
        Poll::Pending
    }
}

fn allowed(multiaddr: &Multiaddr, ips: &HashSet<IpAddr>) -> bool {
    let ip = match multiaddr.iter().next() {
        Some(Protocol::Ip4(ip4)) => IpAddr::V4(ip4),
        Some(Protocol::Ip6(ip6)) => IpAddr::V6(ip6),
        _other => return false,
    };

    ips.contains(&ip)
}

#[derive(Debug)]
pub struct NotAllowed {
    peer: PeerId,
    addr: Multiaddr,
}

impl fmt::Display for NotAllowed {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "peer {} (at addr {}) is not in the allow list",
            self.peer, self.addr
        )
    }
}

impl std::error::Error for NotAllowed {}
