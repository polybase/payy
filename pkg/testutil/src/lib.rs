pub mod eth;

use std::collections::HashSet;

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
