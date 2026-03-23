mod docker;
mod util;

pub use docker::{DockerPostgres, FixtureError, Result};
pub use util::docker_available;
