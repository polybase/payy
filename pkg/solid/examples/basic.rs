use futures::StreamExt;
use solid::App;
use solid::Solid;
use solid::config::SolidConfig;
use solid::event::SolidEvent;
use solid::proposal::{Manifest, ManifestContent};
use solid::test::app::{InsecurePeerSigner, TestApp, UncheckedPeerId as PeerId};
use solid::{Peer, PeerSigner};
use std::time::Duration;
use std::vec;
use tracing::info;
use tracing_subscriber::{filter::EnvFilter, layer::SubscriberExt};

#[tokio::main]
async fn main() {
    let local_peer_id = InsecurePeerSigner::new(PeerId::random());

    // Logging
    let stdout_tracer = tracing_subscriber::fmt::layer().compact();
    let filter = EnvFilter::try_new("warn")
        .unwrap()
        .add_directive("basic=info".parse().unwrap());
    let subscriber = tracing_subscriber::registry()
        .with(stdout_tracer)
        .with(filter);

    tracing::subscriber::set_global_default(subscriber).unwrap();

    // Create a new solid instance
    let mut solid: Solid<TestApp> = Solid::genesis(
        local_peer_id.clone(),
        vec![local_peer_id.peer()],
        TestApp,
        SolidConfig::default(),
    );

    // Start the service
    solid.run();

    // Start
    loop {
        tokio::select! {
            Some(event) = solid.next() => {
                match event {
                    // Node should send accept for an active proposal
                    // to another peer
                    SolidEvent::Accept { accept } => {
                        info!(height = &accept.proposal.height, skips = &accept.skips, to = &accept.leader_id.prefix(), hash = accept.proposal.hash.to_string(), "Send accept");
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

                        // Create the proposal manifest
                        let manifest = Manifest::new(ManifestContent {
                            last_proposal_hash: last_proposal_hash.clone(),
                            skips,
                            height,
                            leader_id: local_peer_id.peer().clone(),
                            state: 0.into(),
                            validators: vec![local_peer_id.peer().clone()],
                            accepts,
                        }, vec![]);
                        let proposal_hash = TestApp::hash(&manifest);

                        info!(hash = proposal_hash.to_string(), height = height, skips = skips, "Propose");

                        // Add proposal to own register, this will trigger an accept
                        solid.receive_proposal(manifest.clone()).unwrap();
                    }

                    // Commit a confirmed proposal changes
                    SolidEvent::Commit { manifest, .. } => {
                        info!(hash = TestApp::hash(&manifest).to_string(), height = manifest.height, skips = manifest.skips, "Commit");
                    }

                    SolidEvent::OutOfSync {
                        height,
                        max_seen_height,
                    } => {
                        info!(local_height = height, max_seen_height = max_seen_height, "Out of sync");
                    }

                    SolidEvent::DuplicateProposal { proposal_hash } => {
                        info!(hash = proposal_hash.to_string(), "Duplicate proposal");
                    }
                }
            }
        }
    }
}
