use libp2p::{
    PeerId, Transport,
    core::{muxing::StreamMuxerBox, transport::Boxed, upgrade},
    dns::TokioDnsConfig,
    identity::Keypair,
    noise, tcp, yamux,
};

/// Create the transports for the swarm, we use TCP/IP and quic.
pub fn create_transport(keypair: &Keypair) -> Boxed<(PeerId, StreamMuxerBox)> {
    // Set up an encrypted DNS-enabled TCP Transport over the yamux protocol.
    let tcp_transport = tcp::tokio::Transport::new(tcp::Config::default().nodelay(true))
        .upgrade(upgrade::Version::V1Lazy)
        .authenticate(noise::Config::new(keypair).unwrap())
        .multiplex(yamux::Config::default())
        .timeout(std::time::Duration::from_secs(20))
        .boxed();

    TokioDnsConfig::system(tcp_transport).unwrap().boxed()
}
