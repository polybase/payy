use std::time::{Duration, Instant};

use async_trait::async_trait;
use doomslug::ApprovalContent;
use primitives::tick_worker::TickWorkerTick;
use tracing::{error, warn};

use crate::{Mode, NodeSharedArc, types::BlockHeight};

#[async_trait]
impl TickWorkerTick for NodeSharedArc {
    async fn tick(&self) -> Option<Instant> {
        let node = &self.0;

        // Get current height
        let height = node.block_cache.lock().height();
        let target_height = height + BlockHeight(1);

        // Check if I am the single validator
        let is_validator = match node.is_validator_for_height(target_height) {
            Ok(value) => value,
            Err(err) => {
                error!(?err, "Unable to determine validator status");
                // Retry soon to avoid busy-looping on configuration errors.
                return Some(Instant::now() + Duration::from_secs(5));
            }
        };

        if !is_validator {
            // This block may mean we can commit some proposals
            loop {
                let next = node.block_cache.lock().get_next_commit_block();

                let Some(block) = next else {
                    break;
                };
                if let Err(err) = node.validate_block(&block).await {
                    error!(?err, ?block, "Error validating block");
                    node.block_cache.lock().remove(&block.hash());
                    continue;
                };
                node.block_cache.lock().confirm(block.content.header.height);
                match node.commit_proposal(block.clone()) {
                    Ok(_) => {}
                    Err(err) => {
                        error!("Unable to commit proposal: {}", err);
                        return None;
                    }
                }
            }

            if node.is_out_of_sync() {
                if let Err(err) = node.handle_out_of_sync().await {
                    error!(?err, "Error syncing");
                };

                // Try again in 5 seconds
                return Some(Instant::now() + Duration::from_secs(5));
            }

            // We're in validator mode, but we're not currently in the smart contract
            // validator set. Instead if waiting to be awoken,
            if node.config.mode == Mode::Validator {
                // Warn in case of mis-configuration
                warn!(
                    address = node.self_peer().to_hex(),
                    "Node running in validator mode, but is not a designated validator: {}",
                    node.self_peer().to_hex()
                );

                // I'm not a designated validator on master chain, so wait for 60 seconds
                return Some(Instant::now() + Duration::from_secs(60));
            }

            // Ticker will no longer be called, we need to awake it later
            // with ticker.tick()
            return None;
        }

        let last_confirmed = *node.block_cache.lock().hash();
        let start_time = Instant::now();

        let approval = ApprovalContent::new_endorsement(&last_confirmed, target_height.0)
            .to_approval_validated(&node.local_peer);

        match node
            .create_proposal(last_confirmed, BlockHeight(height.0 + 1), vec![approval])
            .await
        {
            Ok(_) => {}
            Err(err) => {
                error!("Error creating proposal: {}", err)
            }
        }

        let next_time: Instant = start_time + Duration::from_secs(1);
        Some(next_time)
    }
}
