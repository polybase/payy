pub mod eth;
mod wait_for;

use std::collections::HashSet;
pub use wait_for::wait_for;

pub const ACCOUNT_1_SK: &str = "ac0974bec39a17e36ba4a6b4d238ff944bacb478cbed5efcae784d7bf4f2ff80";

pub struct PortPool {
    ports: HashSet<u16>,
}

impl PortPool {
    pub fn new(range: std::ops::Range<u16>) -> Self {
        Self {
            ports: range.collect(),
        }
    }

    pub fn get(&mut self) -> u16 {
        let port = *self.ports.iter().next().expect("No ports left");
        self.ports.remove(&port);
        port
    }

    pub fn release(&mut self, port: u16) {
        self.ports.insert(port);
    }
}
