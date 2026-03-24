// lint-long-file-override allow-max-lines=300
use crate::{
    Error,
    behaviour::{Behaviour, BehaviourEvent},
    command::Command,
    error::Result,
    protocol::{PolyProtocol, Request, Response},
    transport::create_transport,
};
use borsh::{BorshDeserialize, BorshSerialize};
use futures_util::StreamExt;
use libp2p::{
    Multiaddr, PeerId,
    identity::Keypair,
    request_response,
    swarm::{SwarmBuilder, SwarmEvent, keep_alive},
};
use parking_lot::Mutex;
use std::{
    collections::{HashMap, HashSet},
    fmt::Debug,
    marker::PhantomData,
};
use std::{net::IpAddr, sync::Arc};
use tokio::{select, sync::Mutex as AsyncMutex, sync::mpsc, sync::oneshot};
use tracing::{debug, error, info};

pub struct Network<NetworkEvent>
where
    NetworkEvent: Debug + Clone + Send + BorshSerialize + BorshDeserialize + 'static,
{
    netin_rx: AsyncMutex<mpsc::UnboundedReceiver<(PeerId, NetworkEvent)>>,
    netout_tx: mpsc::UnboundedSender<Command<NetworkEvent>>,
    local_peer_id: PeerId,
    shared: Arc<NetworkShared>,
}

impl<NetworkEvent> Network<NetworkEvent>
where
    NetworkEvent: Debug + Clone + Sync + Send + BorshSerialize + BorshDeserialize + 'static,
{
    pub fn new(
        keypair: &Keypair,
        listenaddrs: impl Iterator<Item = Multiaddr>,
        dialaddrs: impl Iterator<Item = Multiaddr>,
        whitelisted_ips: HashSet<IpAddr>,
    ) -> Result<Network<NetworkEvent>> {
        let local_peer_id = PeerId::from(keypair.public());
        let transport = create_transport(keypair);
        let protocols = vec![(
            PolyProtocol(PhantomData),
            request_response::ProtocolSupport::Full,
        )];
        let rr_config = request_response::Config::default();
        let mut swarm = {
            let whitelist = {
                if whitelisted_ips.is_empty() {
                    None
                } else {
                    let mut whitelist_config = whitelist_ips::Config::default();
                    whitelist_config.whitelisted_ips = whitelisted_ips;
                    let whitelist_ips = whitelist_ips::Behaviour::new_with_config(whitelist_config);
                    Some(whitelist_ips)
                }
            }
            .into();

            let behaviour = Behaviour {
                rr: request_response::Behaviour::new(
                    PolyProtocol(PhantomData),
                    protocols,
                    rr_config,
                ),
                keep_alive: keep_alive::Behaviour,
                whitelist,
            };
            SwarmBuilder::with_tokio_executor(transport, behaviour, local_peer_id).build()
        };

        // Listen on given addresses
        for addr in listenaddrs {
            swarm.listen_on(addr)?;
        }

        // Connect to peers
        for addr in dialaddrs {
            info!(addr = ?addr, "Dialing peer");
            swarm.dial(addr)?;
        }

        // Channel to receive NetworkEvents from the network
        let (netin_tx, netin_rx) = mpsc::unbounded_channel::<(PeerId, NetworkEvent)>();
        let (netout_tx, mut netout_rx) = mpsc::unbounded_channel::<Command<NetworkEvent>>();

        // Shared state between the network and the spawned network behaviour event loop
        let shared: Arc<NetworkShared> = Arc::new(NetworkShared::new());
        let shared_clone = Arc::clone(&shared);

        tokio::spawn(async move {
            let shared = shared_clone;
            let mut requests = HashMap::new();

            // TODO: add cancel loop
            loop {
                select! {
                    Some(cmd) = netout_rx.recv() => {
                        match cmd {
                            Command::Send(peer_id, event, response) => {
                                let request_id = swarm.behaviour_mut().rr.send_request(&peer_id, Request::V1 ( event ));
                                requests.insert(request_id, response);
                            }
                            Command::Dial(peer_id, response) => {
                                response.send(swarm.dial(peer_id)).ok();
                            }
                        }
                    }
                    event = swarm.select_next_some() => match event {
                        SwarmEvent::NewListenAddr { address, .. } => {
                            info!(addr = ?address, "Listening on");
                        }
                        SwarmEvent::Dialing(peer_id) => {
                            info!(peer_id = ?peer_id, "Dialing peer");
                        }
                        SwarmEvent::ConnectionEstablished { peer_id, established_in, .. } => {
                            info!(peer_id = ?peer_id, established_in = ?established_in, "Connection established");
                            shared.add_peer(peer_id);
                        }
                        SwarmEvent::ConnectionClosed { peer_id, endpoint, num_established, cause } => {
                            info!(peer_id = ?peer_id, num_established = num_established, endpoint = ?endpoint, cause = ?cause, "Connection closed");
                            shared.remove_peer(&peer_id);
                        }
                        SwarmEvent::IncomingConnection { local_addr, send_back_addr } => {
                            info!(local_addr = ?local_addr, send_back_addr = ?send_back_addr, "Incoming connection");
                        }
                        SwarmEvent::IncomingConnectionError { local_addr, send_back_addr, error } => {
                            error!(local_addr = ?local_addr, send_back_addr = ?send_back_addr, err = ?error, "Incoming connection error");
                        }
                        SwarmEvent::OutgoingConnectionError { peer_id, error } => {
                            error!(peer_id = ?peer_id, err = ?error, "Outgoing connection error");
                        }
                        SwarmEvent::ListenerClosed { listener_id, addresses, reason } => {
                            error!(listener_id = ?listener_id, addresses = ?addresses, reason = ?reason, "Listener closed");
                        }
                        SwarmEvent::ListenerError { listener_id, error } => {
                            error!(listener_id = ?listener_id, err = ?error, "Listener error");
                        }
                        SwarmEvent::Behaviour(BehaviourEvent::Rr(request_response::Event::Message { peer, message })) => {
                            match message {
                                request_response::Message::Response{ request_id, .. } => {
                                    // Notify sender that request/response process is complete
                                    if let Some(tx) = requests.remove(&request_id) {
                                        tx.send(()).ok();
                                    }
                                },
                                request_response::Message::Request{ request: Request::V1(request), channel, .. } => {
                                        match netin_tx.send((peer, request)) {
                                            Ok(_) => {},
                                            Err(err) => {
                                                error!(?err, peer_id = ?peer, "Failed to send, dropping event");
                                            }
                                        }
                                        match swarm.behaviour_mut().rr.send_response(channel, Response::V1) {
                                            Ok(_) => {},
                                            Err(err) => {
                                                error!(?err, peer_id = ?peer,  "Failed to send response");
                                            }
                                        }
                                }
                           }
                        }
                        SwarmEvent::Behaviour(BehaviourEvent::Rr(request_response::Event::ResponseSent { .. })) => {}
                        event => {
                            debug!(event = ?event, "Swarm event");
                        }
                    }
                }
            }
        });

        Ok(Network {
            netin_rx: AsyncMutex::new(netin_rx),
            netout_tx,
            local_peer_id,
            shared,
        })
    }

    pub async fn dial(&self, peer: Multiaddr) -> Result<()> {
        let (tx, rx) = oneshot::channel();

        self.netout_tx
            .send(Command::Dial(peer, tx))
            .map_err(|err| Error::ChannelError(err.to_string()))?;

        let res = rx
            .await
            .map_err(|err| Error::ChannelError(err.to_string()))?;

        Ok(res?)
    }

    pub async fn send(&self, peer: &PeerId, event: NetworkEvent) {
        self._send(peer, event).await;
    }

    pub async fn send_all(&self, event: NetworkEvent) {
        let peers = self.shared.state.lock().connected_peers.clone();
        let mut futures = vec![];

        for peer in peers.iter() {
            futures.push(self._send(peer, event.clone()));
        }

        futures::future::join_all(futures).await;
    }

    async fn _send(&self, peer: &PeerId, event: NetworkEvent) -> Option<oneshot::Receiver<()>> {
        // Don't send messages to self
        if self.local_peer_id == *peer {
            return None;
        }

        let (tx, rx) = oneshot::channel();

        match self.netout_tx.send(Command::Send(*peer, event, tx)) {
            Ok(_) => {}
            Err(err) => {
                error!(?err, peer_id = ?peer, "Failed to send, dropping event");
            }
        }

        Some(rx)
    }

    pub async fn next(&self) -> Option<(PeerId, NetworkEvent)> {
        self.netin_rx.lock().await.recv().await
    }
}

struct NetworkShared {
    state: Mutex<NetworkSharedState>,
}

impl NetworkShared {
    fn new() -> NetworkShared {
        NetworkShared {
            state: Mutex::new(NetworkSharedState {
                connected_peers: HashSet::new(),
            }),
        }
    }

    fn add_peer(&self, peer_id: PeerId) {
        self.state.lock().connected_peers.insert(peer_id);
    }

    fn remove_peer(&self, peer_id: &PeerId) {
        self.state.lock().connected_peers.remove(peer_id);
    }
}

struct NetworkSharedState {
    connected_peers: HashSet<PeerId>,
}
