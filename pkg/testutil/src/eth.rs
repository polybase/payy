use std::{
    path::PathBuf,
    process::Command,
    sync::{Arc, Mutex},
};

use once_cell::sync::Lazy;

use crate::PortPool;

fn find_eth() -> PathBuf {
    let mut path = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    path.push("../../eth");
    path
}

static PORT_POOL: Lazy<Mutex<PortPool>> =
    once_cell::sync::Lazy::new(|| Mutex::new(PortPool::new(13001..13001 + 1000)));

#[derive(Debug)]
pub struct EthNode {
    process: Option<std::process::Child>,
    port: u16,
}

impl Drop for EthNode {
    fn drop(&mut self) {
        self.stop();
        PORT_POOL.lock().unwrap().release(self.port);
    }
}

impl EthNode {
    fn new() -> Self {
        let port = PORT_POOL.lock().unwrap().get();

        Self {
            process: None,
            port,
        }
    }

    fn run(&mut self) {
        // This must be the actual hardhat bin instead of running it through yarn,
        // because we send a SIGKILL which yarn can't forward to the hardhat node.
        let mut command = Command::new("node_modules/.bin/hardhat");

        command.current_dir(find_eth());

        command.arg("node");
        command.arg("--port").arg(self.port.to_string());

        let should_log = std::env::var("LOG_HARDHAT_OUTPUT")
            .map(|v| v == "1")
            .unwrap_or(false);
        if !should_log {
            command
                .stdout(std::process::Stdio::null())
                .stderr(std::process::Stdio::null());
        }

        let process = command.spawn().expect("Failed to start hardhat node");
        self.process = Some(process);
    }

    fn stop(&mut self) {
        if let Some(mut process) = self.process.take() {
            process.kill().expect("Failed to kill hardhat node");
            process
                .wait()
                .expect("Failed to wait for hardhat node to exit");
        }
    }

    pub fn rpc_url(&self) -> String {
        format!("http://127.0.0.1:{}", self.port)
    }

    async fn wait_for_healthy(&self) -> Result<(), Box<dyn std::error::Error>> {
        let time_between_requests = std::time::Duration::from_millis(100);
        let max_retries = 10_000 / time_between_requests.as_millis() as usize;

        let mut retry = 0;
        loop {
            let is_last_retry = retry == max_retries - 1;

            let req = reqwest::Client::new().get(self.rpc_url()).build().unwrap();

            match reqwest::Client::new().execute(req).await {
                Ok(res) if res.status().is_success() => return Ok(()),
                Ok(res) if is_last_retry => {
                    return Err(format!("Failed to get health: {}", res.status()).into())
                }
                Ok(_) => {}
                Err(err) if is_last_retry => return Err(err.into()),
                Err(_) => {}
            }

            tokio::time::sleep(time_between_requests).await;
            retry += 1;
        }
    }

    async fn wait(&self) -> Result<(), Box<dyn std::error::Error>> {
        self.wait_for_healthy().await?;
        tokio::time::sleep(std::time::Duration::from_secs(1)).await;
        self.wait_for_healthy().await?;

        Ok(())
    }

    fn deploy(&self) -> Result<(), Box<dyn std::error::Error>> {
        let mut command = Command::new("node_modules/.bin/hardhat");

        command.current_dir(find_eth());

        command.env(
            "SECRET_KEY",
            "0xac0974bec39a17e36ba4a6b4d238ff944bacb478cbed5efcae784d7bf4f2ff80",
        );
        command.env("TESTING_URL", self.rpc_url());

        command.arg("run");
        command.arg("scripts/deploy.ts");
        command.arg("--network").arg("testing");

        let should_log = std::env::var("LOG_HARDHAT_DEPLOY_OUTPUT")
            .map(|v| v == "1")
            .unwrap_or(false);
        if !should_log {
            command
                .stdout(std::process::Stdio::null())
                .stderr(std::process::Stdio::null());
        }

        let mut process = command.spawn().expect("Failed to start hardhat deploy");
        let status = process.wait()?;

        if !status.success() {
            Err("hardhat deploy returned a non-zero exit code".into())
        } else {
            Ok(())
        }
    }

    pub async fn run_and_deploy() -> Arc<Self> {
        let mut eth_node = Self::new();

        let eth_node = tokio::task::spawn_blocking(move || {
            eth_node.run();
            eth_node
        })
        .await
        .unwrap();

        eth_node.wait().await.expect("Failed to wait for eth node");

        let eth_node = tokio::task::spawn_blocking(move || {
            // Deploy is flaky
            for i in 0..3 {
                match eth_node.deploy() {
                    Ok(_) => break,
                    Err(err) => {
                        if i == 2 {
                            panic!("Failed to deploy contracts: {err:?}");
                        } else {
                            std::thread::sleep(std::time::Duration::from_secs(5));
                        }
                    }
                }
            }

            eth_node
        })
        .await
        .unwrap();

        Arc::new(eth_node)
    }
}
