use std::{path::Path, sync::OnceLock};

use block_store::BlockStore;
use contextful::ResultContextExt as _;
use element::Element;
use tracing::info;

use crate::{
    BlockFormat, Node, NodeShared, PersistentMerkleTree, Result, block::Block, config::Config,
    constants::MERKLE_TREE_DEPTH, types::BlockHeight,
};

pub(super) struct LoadedData {
    pub block_store: BlockStore<BlockFormat>,
    pub persistent_tree: PersistentMerkleTree,
    pub block: Block,
}

fn empty_tree_hash() -> Element {
    static EMPTY_TREE_HASH: OnceLock<Element> = OnceLock::new();
    *EMPTY_TREE_HASH.get_or_init(|| smirk::Tree::<MERKLE_TREE_DEPTH, ()>::default().root_hash())
}

impl Node {
    #[tracing::instrument(skip_all)]
    pub(super) fn load_db_and_smirk(config: &Config) -> Result<LoadedData> {
        let db_path = &config.db_path.join("latest");
        info!("Loading DB from: {}", db_path.to_str().unwrap());

        let smirk_path = config.smirk_path.join("latest");
        info!("Loading Smirk from: {}", &smirk_path.to_str().unwrap());

        let block_store = BlockStore::create_or_load(db_path)
            .context("load block store from latest database path")?;
        let mut persistent_tree = smirk::storage::Persistent::load(&smirk_path)
            .context("load persistent smirk tree from latest path")?;

        let Some(max_height) = block_store
            .get_max_height()
            .context("fetch max block height from block store")?
        else {
            info!(
                smirk_root_hash = ?persistent_tree.tree().root_hash(),
                "No blocks found in the block store, resetting smirk and starting from genesis"
            );

            drop(persistent_tree);

            Self::reset_db_and_smirk(None, Some(&config.smirk_path))?;
            let persistent_tree = smirk::storage::Persistent::load(&smirk_path)
                .context("reload persistent smirk tree after reset")?;

            debug_assert_eq!(persistent_tree.tree().root_hash(), empty_tree_hash());

            let data = LoadedData {
                block_store,
                persistent_tree,
                block: Block::genesis(),
            };

            return Ok(data);
        };

        let block = block_store
            .get(max_height)
            .context("fetch latest block from block store")?
            .unwrap()
            .into_block();

        if persistent_tree.tree().root_hash() == block.content.state.root_hash {
            let data = LoadedData {
                block_store,
                persistent_tree,
                block,
            };

            return Ok(data);
        }

        let previous_block = block_store
            .get(BlockHeight(max_height.0 - 1))
            .context("fetch previous block during crash recovery")?
            .unwrap()
            .into_block();

        if persistent_tree.tree().root_hash() == previous_block.content.state.root_hash {
            info!(
                local_tree_root_hash = ?persistent_tree.tree().root_hash(),
                block_root_hash = ?block.content.state.root_hash,
                previous_block_root_hash = ?previous_block.content.state.root_hash,
                "The node crashed after committing to block store, but before committing to notes tree. We will recover by applying the block to the tree."
            );

            NodeShared::apply_block_to_tree(
                &mut persistent_tree,
                &block.content.state,
                max_height,
            )?;
            assert!(persistent_tree.tree().root_hash() == block.content.state.root_hash);

            let data = LoadedData {
                block_store,
                persistent_tree,
                block,
            };

            return Ok(data);
        }

        info!(
            local_tree_root_hash = ?persistent_tree.tree().root_hash(),
            block_root_hash = ?block.content.state.root_hash,
            previous_block_root_hash = ?previous_block.content.state.root_hash,
            "Block store and tree are too far out of sync to recover. Resetting and starting from genesis"
        );
        drop(block_store);
        drop(persistent_tree);

        Self::reset_db_and_smirk(Some(&config.db_path), Some(&config.smirk_path))?;

        let block_store =
            BlockStore::create_or_load(db_path).context("reload block store after full reset")?;
        let persistent_tree = smirk::storage::Persistent::load(&smirk_path)
            .context("reload persistent smirk tree after full reset")?;

        let max_height = block_store
            .get_max_height()
            .context("verify block store empty after reset")?;
        debug_assert!(max_height.is_none());
        debug_assert_eq!(persistent_tree.tree().root_hash(), empty_tree_hash(),);

        let data = LoadedData {
            block_store,
            persistent_tree,
            block: Block::genesis(),
        };

        Ok(data)
    }

    /// Moves current db and smirk to old-{unix-timestamp-millis}-{random}
    #[tracing::instrument(skip_all)]
    fn reset_db_and_smirk(db_path: Option<&Path>, smirk_path: Option<&Path>) -> Result<()> {
        let timestamp = chrono::Utc::now().timestamp_millis();
        let random = rand::random::<u32>();
        let new_dir_name = format!("old-{timestamp}-{random}");

        if let Some(db_path) = db_path {
            let new_db_path = db_path.join(&new_dir_name);
            std::fs::rename(db_path.join("latest"), &new_db_path)
                .context("rename latest db directory into timestamped backup")?;
            info!("Moved db to {:?}", new_db_path);
        }

        if let Some(smirk_path) = smirk_path {
            let new_smirk_path = smirk_path.join(&new_dir_name);
            std::fs::rename(smirk_path.join("latest"), &new_smirk_path)
                .context("rename latest smirk directory into timestamped backup")?;
            info!("Moved smirk to {:?}", new_smirk_path);
        }

        Ok(())
    }
}
