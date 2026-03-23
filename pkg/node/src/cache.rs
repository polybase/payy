// lint-long-file-override allow-max-lines=400
use std::collections::{BTreeMap, HashMap};

use primitives::hash::CryptoHash;

use crate::{block::Block, types::BlockHeight};

/// BlockCache for last commit and all other uncomitted blocks. Stores a given maximum
//  of cached blocks, so if we receive blocks out of order we don't need to do an extra sync.
#[derive(Debug)]
pub struct BlockCache {
    height: BlockHeight,
    max_height: BlockHeight,
    blocks: HashMap<CryptoHash, Block>,
    block_hash_heights: BTreeMap<BlockHeight, CryptoHash>,
    max_cache_size: usize,
}

impl BlockCache {
    pub fn new(start_block: Block, max_cache_size: usize) -> Self {
        let start_height = start_block.content.header.height;
        let mut cache = BlockCache {
            height: start_height,
            max_height: start_height,
            blocks: HashMap::new(),
            block_hash_heights: BTreeMap::new(),
            max_cache_size,
        };

        // Always ensure we have the confirmed height block
        cache.insert(start_block);

        cache
    }

    pub fn height(&self) -> BlockHeight {
        self.height
    }

    pub fn max_height(&self) -> BlockHeight {
        self.max_height
    }

    pub fn hash(&self) -> &CryptoHash {
        self.block_hash_heights.get(&self.height()).unwrap()
    }

    pub fn insert(&mut self, block: Block) {
        let height = block.content.header.height;
        let block_hash = block.content.hash();

        // Don't add old blocks
        if self.height() > height {
            return;
        }

        // Keep track of the maximum height we have seen
        if height > self.max_height {
            self.max_height = height;
        }

        let (max_height, max_hash) = self
            .block_hash_heights
            .iter()
            .last()
            .map(|(k, v)| (*k, *v))
            .unwrap_or((BlockHeight(0), CryptoHash::default()));

        let cache_at_capacity = self.blocks.len() >= self.max_cache_size;

        // Don't insert blocks if blocks are greater than our maximum cache size, this is
        // to prevent the cache from growing too large and consuming too much memory.
        if cache_at_capacity && height > max_height {
            return;
        }

        if cache_at_capacity {
            self.remove(&max_hash);
        }

        self.block_hash_heights.insert(height, block_hash);
        self.blocks.insert(block_hash, block);
    }

    /// Confirm a proposal, all subsequent proposals must now
    /// include this proposal in the tree.
    pub fn confirm(&mut self, height: BlockHeight) {
        self.height = height;

        // Remove all proposals less than this
        let mut heights_to_remove = Vec::new();

        // Collect heights of all blocks less than the confirmed height
        for (&height_key, _) in self.block_hash_heights.range(..height) {
            heights_to_remove.push(height_key);
        }

        // Remove blocks with height less than the confirmed height
        for &height in &heights_to_remove {
            if let Some(hash) = self.block_hash_heights.remove(&height) {
                self.blocks.remove(&hash);
            }
        }
    }

    pub fn get_range(&self, start: BlockHeight, end: BlockHeight) -> Vec<&Block> {
        self.block_hash_heights
            .range(start..end)
            .filter_map(|(_, h)| self.blocks.get(h))
            .collect()
    }

    pub fn get_by_height(&self, height: BlockHeight) -> Option<&Block> {
        self.block_hash_heights
            .get(&height)
            .and_then(|h| self.blocks.get(h))
    }

    pub fn remove(&mut self, hash: &CryptoHash) {
        if let Some(block) = self.blocks.remove(hash) {
            self.block_hash_heights.remove(&block.content.header.height);
        }
    }

    #[must_use]
    pub fn is_out_of_sync(&self) -> bool {
        // We may have removed later proposals from the cache (if we are
        // far behind the network) to make space for earlier ones
        if self.max_height
            > *self
                .block_hash_heights
                .iter()
                .last()
                .map(|(key, _)| key)
                .unwrap()
        {
            return true;
        }

        let range = (self.height.0 + 1)..=self.max_height.0;

        // Start at the next block after the last confirmed height.
        // Iterate till max height.
        for height in range.map(BlockHeight) {
            // If the block hash does not exist for this height, return true.
            if !self.block_hash_heights.contains_key(&height) {
                return true;
            }
        }

        // If there are no missing blocks, return false.
        false
    }

    // If we have a valid chain + 2 then we can commit the latest uncommited block
    pub fn get_next_commit_block(&mut self) -> Option<Block> {
        let next_height = self.height + BlockHeight(1);

        // if we have the next block, confirm it
        match self.get_by_height(next_height) {
            Some(block) => {
                let b = block.clone();
                Some(b)
            }
            None => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use element::Element;
    use primitives::sig::Signature;

    use crate::block::{BlockContent, BlockHeader, BlockState};

    use super::*;

    fn block(height: u64) -> Block {
        Block {
            content: BlockContent {
                header: BlockHeader {
                    height: BlockHeight(height),
                    epoch_id: 0,
                    last_block_hash: CryptoHash::from_u64(height.saturating_sub(1)),
                    last_final_block_hash: CryptoHash::from_u64(height.saturating_sub(1)),
                    approvals: vec![],
                },
                state: BlockState {
                    root_hash: Element::from(height),
                    txns: vec![],
                },
            },
            signature: Signature::default(),
        }
    }

    #[test]
    fn test_maximum_cache() {
        let mut block_cache = BlockCache::new(block(0), 10);

        // Inserting mock data with different heights
        for i in 20..22 {
            let block = block(i);
            block_cache.insert(block);
        }

        assert!(block_cache.get_by_height(20.into()).is_some());
        assert!(block_cache.get_by_height(21.into()).is_some());

        // Inserting mock data with different heights
        for i in 0..13 {
            let block = block(i);
            block_cache.insert(block);
        }

        assert!(block_cache.get_by_height(0.into()).is_some());
        assert!(block_cache.get_by_height(9.into()).is_some());
        assert!(block_cache.get_by_height(12.into()).is_none());

        assert_eq!(block_cache.block_hash_heights.len(), 10);
        assert_eq!(block_cache.blocks.len(), 10);
    }

    #[test]
    fn test_is_out_of_sync() {
        let mut block_cache = BlockCache::new(block(0), 10);

        // Inserting mock data with different heights
        for i in 0..6 {
            let block = block(i);
            block_cache.insert(block);
        }

        // After adding continuous blocks from 0 to 5, cache should not be out of sync
        assert!(!block_cache.is_out_of_sync());

        // Simulate a gap in proposals
        for i in 9..12 {
            let block = block(i);
            block_cache.insert(block);
        }

        // After removing a block, the cache should be out of sync
        assert!(block_cache.is_out_of_sync());
    }

    #[test]
    fn test_confirm() {
        let mut block_cache = BlockCache::new(block(0), 10);

        // Inserting mock data with different heights
        for i in 0..6 {
            let block = block(i);
            block_cache.insert(block);
        }

        // Confirming the height to 3
        block_cache.confirm(3.into());

        // Check `height` is updated to confirmed height
        assert_eq!(block_cache.height(), BlockHeight(3));

        // Check blocks and hashes below the confirmed height are removed
        assert!(block_cache.blocks.len() == 3);
        assert!(block_cache.block_hash_heights.len() == 3);

        for i in 0..3 {
            assert!(block_cache.get_by_height(i.into()).is_none());
            assert!(!block_cache.block_hash_heights.contains_key(&BlockHeight(i)));
        }

        // Check blocks and hashes above or equal to the confirmed height are untouched
        for i in 3..6 {
            assert!(block_cache.get_by_height(i.into()).is_some());
            assert!(block_cache.block_hash_heights.contains_key(&BlockHeight(i)));
        }
    }

    #[test]
    fn test_get_next_commit_block() {
        let mut block_cache = BlockCache::new(block(0), 10);

        // Inserting mock data with different heights
        for i in 0..3 {
            let block = block(i);
            block_cache.insert(block);
        }

        // The chain is not out of sync, we should be able to commit block 1 from height 0
        let next_commit_block = block_cache.get_next_commit_block().unwrap();
        assert_eq!(next_commit_block.content.header.height, BlockHeight(1));

        // Simulate a gap in proposals
        for i in 9..12 {
            let block = block(i);
            block_cache.insert(block);
        }

        // The chain is now out of sync, we should not be able to commit any block
        // assert!(block_cache.get_next_commit_block().is_none());
    }
}
