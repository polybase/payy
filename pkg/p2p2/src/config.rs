use std::{collections::HashSet, net::IpAddr, path::Path, sync::OnceLock};

use figment::{
    Figment,
    providers::{Env, Format, Toml},
};
use libp2p::Multiaddr;
use serde::{Deserialize, Deserializer};

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[non_exhaustive]
#[serde(rename_all = "kebab-case")]
/// Config to configure the P2P node
pub struct Config {
    /// The multiaddr to listen on
    pub laddr: Multiaddr,

    /// A list of other multiaddrs to dial when calling `spawn`,
    #[serde(deserialize_with = "deserialize_multiaddr")]
    pub dial: Vec<Multiaddr>,

    /// The number of seconds to keep a connection alive
    ///
    /// 0 means "no timeout" (i.e. `u64::MAX` seconds)
    pub idle_timeout_secs: u64,

    /// Explicitly whitelisted IP addresses
    ///
    /// Any IP address that isn't in this set will be banned (connections will be immediately
    /// rejected)
    ///
    /// If empty, whitelisting is disabled (i.e. all IPs are allowed)
    pub whitelisted_ips: HashSet<IpAddr>,
}

impl Default for Config {
    fn default() -> Self {
        static DEFAULT_CONFIG: OnceLock<Config> = OnceLock::new();

        let config_ref = DEFAULT_CONFIG.get_or_init(|| toml::from_str(Self::DEFAULT_STR).unwrap());

        config_ref.clone()
    }
}

impl Config {
    /// The text of the default config file
    pub const DEFAULT_STR: &'static str = include_str!("../default_config.toml");

    /// Load a [`Config`] from the given file, while taking into account environment variables
    ///
    /// In particular, specific fields can be overridden by setting the `P2P_*` environment
    /// variable. For example:
    ///  - to override `listen_on`, set `P2P_LISTEN-ON="..."` (notice the kebab case inside an
    ///    identifier)
    ///  - to override `foo.bar`, set `P2P_FOO_BAR="..."`
    ///
    ///  Use underscores to separate identifiers, and hyphens to separate words within an
    ///  identifier
    pub fn from_env<P: AsRef<Path>>(path: P) -> Result<Config, Box<figment::Error>> {
        tracing::info!("loading config from {}", path.as_ref().to_string_lossy());

        Figment::new()
            .merge(Toml::file(path))
            .merge(Env::prefixed("P2P_").split("_"))
            .join(Toml::string(Self::DEFAULT_STR))
            .extract()
            .map_err(Box::new)
    }
}

fn deserialize_multiaddr<'de, D>(deserializer: D) -> Result<Vec<Multiaddr>, D::Error>
where
    D: Deserializer<'de>,
{
    let s = String::deserialize(deserializer)?;
    s.split(',')
        .map(|part| {
            part.trim()
                .parse::<Multiaddr>()
                .map_err(serde::de::Error::custom)
        })
        .collect()
}
