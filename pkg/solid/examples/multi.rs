// lint-long-file-override allow-max-lines=500
use borsh::{BorshDeserialize, BorshSerialize};
use futures::StreamExt;
use futures::channel::mpsc::{self, Sender, TrySendError};
use futures::stream::Stream;
use rand::Rng;
use solid::App;
use solid::Solid;
use solid::config::SolidConfig;
use solid::event::SolidEvent;
use solid::proposal::ManifestContent;
use solid::proposal::{Manifest, ProposalAccept};
use solid::test::app::{InsecurePeerSigner, TestApp, TestAppTxnState, UncheckedPeerId as PeerId};
use solid::{Peer, PeerSigner};
use std::collections::HashMap;
// use std::mem;
use std::pin::Pin;
use std::sync::atomic::AtomicBool;
use std::sync::{Arc, Mutex};
use std::task::{Context, Poll};
use std::time::Duration;
use tokio::time::sleep;

use tracing::{Instrument, error, info, info_span};
use tracing_subscriber::{filter::EnvFilter, layer::SubscriberExt};

#[derive(Debug, BorshSerialize, BorshDeserialize)]
struct Store {
    data: HashMap<String, String>,
    proposal: Option<Manifest<PeerId, TestAppTxnState>>,
    pending: Vec<u64>,
}

impl Store {
    // fn propose(&mut self) -> Vec<solid::txn::Txn> {
    //     mem::take(&mut self.pending)
    // }

    // fn add_pending_txn(&mut self, txn: solid::txn::Txn) {
    //     self.pending.push(txn);
    // }

    fn commit(&mut self, manifest: Manifest<PeerId, TestAppTxnState>) -> Vec<u8> {
        self.proposal = Some(manifest);
        vec![]
    }

    // fn restore(&mut self, snapshot: Snapshot) {
    //     let Snapshot { data, proposal } = snapshot;
    //     let data: HashMap<String, String> = deserialize(&data).unwrap();
    //     self.data = data;
    //     self.proposal = Some(proposal)
    // }

    #[allow(clippy::disallowed_methods)] // it's just an example
    fn snapshot(&self) -> std::result::Result<Snapshot, Box<dyn std::error::Error>> {
        Ok(Snapshot {
            data: borsh::to_vec(&self.data)?,
            proposal: self.proposal.as_ref().unwrap().clone(),
        })
    }
}

pub type SenderMap = HashMap<PeerId, Sender<(PeerId, Vec<u8>)>>;

#[derive(Clone)]
pub struct MyNetworkConfig {
    min_latency: u64,
    max_latency: u64,
    drop_probability: f64,
    partition_duration: u64,
    partition_frequency: u64,
}

pub struct MyNetwork {
    local_peer_id: PeerId,
    event_stream: Option<Box<dyn Stream<Item = (PeerId, Vec<u8>)> + Unpin + Sync + Send>>,
    senders: Arc<Mutex<SenderMap>>,
    partition: Arc<AtomicBool>,
    config: MyNetworkConfig,
}

impl MyNetwork {
    pub fn new(
        local_peer_id: PeerId,
        senders: Arc<Mutex<SenderMap>>,
        config: MyNetworkConfig,
    ) -> Self {
        // A small buffer, so we can simulate network partition
        let (sender, receiver) = mpsc::channel(20);

        // Insert self into senders!
        {
            let mut senders = senders.lock().unwrap();
            senders.insert(local_peer_id.clone(), sender);
        }

        let partition = Arc::new(AtomicBool::new(false));

        let drop_probability = config.drop_probability;
        let dropping_stream = DroppingStream::new(receiver, drop_probability, partition.clone());

        let event_stream = Some(Box::new(dropping_stream)
            as Box<dyn Stream<Item = (PeerId, Vec<u8>)> + Unpin + Send + Sync>);

        let prefix = local_peer_id.prefix();
        let shared_partition = partition.clone();
        tokio::spawn(async move {
            loop {
                if rand::thread_rng().gen_range(0..=config.partition_frequency) == 1 {
                    println!(
                        "----- START: simulating {}s network partition for {} -----",
                        config.partition_duration / 1000,
                        prefix,
                    );
                    {
                        shared_partition.swap(true, std::sync::atomic::Ordering::Relaxed);
                    }
                    sleep(std::time::Duration::from_millis(config.partition_duration)).await;
                    println!("----- END: simulating network partition for {prefix} -----",);
                    {
                        shared_partition.swap(false, std::sync::atomic::Ordering::Relaxed);
                    }
                }

                // Only check for partition every 1 seconds
                sleep(std::time::Duration::from_millis(1000)).await;
            }
        });

        Self {
            local_peer_id,
            event_stream,
            senders,
            config,
            partition,
        }
    }

    async fn send(&self, peer_id: &PeerId, event: &NetworkEvent) {
        #[allow(clippy::disallowed_methods)] // it's just an example
        let data = borsh::to_vec(event).unwrap();

        let senders = self.senders.clone();
        let local_peer_id = self.local_peer_id.clone();
        let MyNetworkConfig {
            min_latency,
            max_latency,
            ..
        } = self.config;
        let sleep_duration =
            rand::thread_rng().gen_range(0..=max_latency - min_latency) + min_latency;

        // If we're simulating a network partition ignore requests
        {
            if self.partition.load(std::sync::atomic::Ordering::Relaxed) {
                return;
            }
        }

        // Randomly sleep for a minute (simulate network partition)
        sleep(std::time::Duration::from_millis(sleep_duration)).await;

        let mut sender_opt = None;
        {
            let senders = senders.lock().unwrap();
            if let Some(sender) = senders.get(peer_id) {
                sender_opt = Some(sender.clone());
            }
        }
        if let Some(mut sender) = sender_opt {
            match sender.try_send((local_peer_id, data)) {
                Ok(_) => {
                    // println!("Sending from {}", self.local_peer_id.prefix());
                }
                Err(TrySendError { .. }) => {
                    info!(
                        "Failed to send message to {} from {}",
                        peer_id.prefix(),
                        self.local_peer_id.prefix(),
                    )
                }
            }
        }
    }

    async fn send_all(&self, event: &NetworkEvent) {
        let senders = self.senders.lock().unwrap().clone();
        let local_peer_id = self.local_peer_id.clone();
        let tasks: Vec<_> = senders
            .keys()
            .filter(|peer_id| **peer_id != local_peer_id) // Exclude local_peer_id from broadcasting
            .map(|peer_id| self.send(peer_id, event))
            .collect();
        futures::future::join_all(tasks).await;
    }
}

impl Stream for MyNetwork {
    type Item = (PeerId, Vec<u8>);

    fn poll_next(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        let stream = self.event_stream.as_mut().unwrap();
        Pin::new(stream).poll_next(cx)
    }
}

#[derive(Debug, Clone, BorshSerialize, BorshDeserialize)]
enum NetworkEvent {
    OutOfSync {
        peer_id: PeerId,
        height: u64,
    },
    Accept {
        accept: ProposalAccept<PeerId>,
    },
    Proposal {
        manifest: Manifest<PeerId, TestAppTxnState>,
    },
    Snapshot {
        snapshot: Snapshot,
    },
}

#[derive(Debug, Clone, PartialEq, Eq, BorshSerialize, BorshDeserialize)]
pub struct Snapshot {
    pub proposal: Manifest<PeerId, TestAppTxnState>,
    pub data: Vec<u8>,
}

#[tokio::main]
async fn main() {
    let num_of_nodes = 3;
    let senders = Arc::new(Mutex::new(HashMap::new()));
    let peers: Vec<PeerId> = (0..num_of_nodes).map(|_| PeerId::random()).collect();

    // Logging
    let stdout_tracer = tracing_subscriber::fmt::layer().compact();

    let filter = EnvFilter::try_new("warn")
        .unwrap()
        .add_directive("multi=info".parse().unwrap());

    let subscriber = tracing_subscriber::registry()
        .with(stdout_tracer)
        .with(filter);

    tracing::subscriber::set_global_default(subscriber).unwrap();

    let mut handles = Vec::new();

    for i in 0..num_of_nodes {
        let local_peer_id = InsecurePeerSigner::new(peers[i].clone());
        let peers = peers.clone();

        let mut store = Store {
            data: HashMap::new(),
            proposal: None,
            pending: vec![],
        };

        info!("Starting node {}", local_peer_id.peer().prefix());

        let config = MyNetworkConfig {
            min_latency: 200,
            max_latency: 600,
            drop_probability: 0.1,
            partition_duration: 80_000,
            partition_frequency: 600,
        };

        let mut network = MyNetwork::new(local_peer_id.peer().clone(), senders.clone(), config);

        let mut solid = Solid::genesis(
            local_peer_id.clone(),
            peers.clone(),
            TestApp,
            SolidConfig::default(),
        );

        let span = info_span!("task_span", local_peer_id = local_peer_id.peer().prefix());
        handles.push(tokio::spawn(async move {
            solid.run();
            loop {
                tokio::select! {
                    Some((_, data)) = network.next() => {
                        let event = NetworkEvent::deserialize(&mut data.as_slice()).unwrap();
                        match event {
                            NetworkEvent::OutOfSync { peer_id, height } => {
                                info!(peer_id = peer_id.prefix(), height = height, "Peer is out of sync");
                                if height + 1024 < solid.height() {
                                    let snapshot = match store.snapshot() {
                                        Ok(snapshot) => snapshot,
                                        Err(err) => {
                                            error!(r#for = peer_id.prefix(), ?err, "Error creating snapshot");
                                            return;
                                        }
                                    };
                                    network.send(&peer_id, &NetworkEvent::Snapshot { snapshot }).await;
                                } else {
                                    for proposal in solid.confirmed_proposals_from(height) {
                                        network.send(
                                            &peer_id,
                                            &NetworkEvent::Proposal {
                                                manifest: proposal.clone(),
                                            },
                                        )
                                        .await;
                                    }
                                }
                            }

                            NetworkEvent::Snapshot { .. } => {
                                info!("Received snapshot");
                                // solid.receive_snapshot(snapshot);
                            }

                            NetworkEvent::Accept { accept } => {
                                info!(height = &accept.proposal.height, skips = &accept.skips, from = &accept.leader_id.prefix(), hash = accept.proposal.hash.to_string(), "Received accept");
                                match solid.receive_accept(&accept) {
                                    Ok(_) => {}
                                    Err(err) => {
                                        error!(?err, "Error receiving accept");
                                    }
                                }
                            }

                            NetworkEvent::Proposal { manifest } => {
                                info!(height = &manifest.height, skips = &manifest.skips, from = &manifest.leader_id.prefix(), hash = TestApp::hash(&manifest).to_string(), "Received proposal");
                                match solid.receive_proposal(manifest) {
                                    Ok(_) => {}
                                    Err(err) => {
                                        error!(?err, "Error receiving proposal");
                                    }
                                }
                            }
                        }
                    },

                    Some(event) = solid.next() => {
                        match event {
                            // Node should send accept for an active proposal
                            // to another peer
                            SolidEvent::Accept { accept } => {
                                info!(height = &accept.proposal.height, skips = &accept.skips, to = &accept.leader_id.prefix(), hash = accept.proposal.hash.to_string(), "Send accept");
                                let leader = &accept.leader_id.clone();

                                network.send(
                                    leader,
                                    &NetworkEvent::Accept { accept },
                                )
                                .await;
                            }

                            // Node should create and send a new proposal
                            SolidEvent::Propose {
                                last_proposal_hash,
                                height,
                                skips,
                                accepts,
                            } => {
                                // Simulate delay
                                tokio::time::sleep(Duration::from_secs(1)).await;

                                // Create the proposl manfiest
                                let manifest = Manifest::new(ManifestContent::<PeerId, TestAppTxnState> {
                                    last_proposal_hash,
                                    skips,
                                    height,
                                    leader_id: local_peer_id.peer().clone(),
                                    state: 0.into(),
                                    validators: peers.clone(),
                                    accepts,
                                }, vec![]);
                                let proposal_hash = TestApp::hash(&manifest);

                                info!(hash = proposal_hash.to_string(), height = height, skips = skips, "Propose");

                                // Add proposal to own register, this will trigger an accept
                                match solid.receive_proposal(manifest.clone()) {
                                    Ok(_) => {}
                                    Err(err) => {
                                        error!(?err, "Error receiving proposal");
                                    }
                                }

                                // // Send proposal to all other nodes
                                network.send_all(
                                    &NetworkEvent::Proposal { manifest: manifest.clone() }
                                )
                                .await;
                            }

                            // Commit a confirmed proposal changes
                            SolidEvent::Commit { manifest, .. } => {
                                info!(hash = TestApp::hash(&manifest).to_string(), height = manifest.height, skips = manifest.skips,  "Commit");
                                store.commit(manifest);
                            }

                            SolidEvent::OutOfSync {
                                height,
                                max_seen_height,
                            } => {
                                info!(local_height = height, max_seen_height = max_seen_height, "Out of sync");
                                network.send_all(&NetworkEvent::OutOfSync { peer_id: local_peer_id.peer().clone(), height })
                                .await
                            }

                            SolidEvent::DuplicateProposal { proposal_hash } => {
                                info!(hash = proposal_hash.to_string(), "Duplicate proposal");
                            }
                        }
                    }
                }
            }
        }.instrument(span)));
    }

    for handle in handles {
        handle.await.unwrap();
    }
}

/// Util for simulating dropping of messages
pub struct DroppingStream<S> {
    inner: S,
    drop_probability: f64,
    partition: Arc<AtomicBool>,
}

impl<S> DroppingStream<S> {
    pub fn new(inner: S, drop_probability: f64, partition: Arc<AtomicBool>) -> Self {
        Self {
            inner,
            drop_probability,
            partition,
        }
    }
}

impl<S: Stream + Unpin> Stream for DroppingStream<S> {
    type Item = S::Item;

    fn poll_next(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        loop {
            let res = self.inner.poll_next_unpin(cx);
            match res {
                Poll::Pending => return Poll::Pending,
                Poll::Ready(Some(item)) => {
                    if rand::thread_rng().gen_range(0.0..=1.0) < self.drop_probability
                        || self.partition.load(std::sync::atomic::Ordering::Relaxed)
                    {
                        info!("Dropping message");
                        continue;
                    }
                    return Poll::Ready(Some(item));
                }
                Poll::Ready(None) => return Poll::Ready(None),
            }
        }
    }
}
