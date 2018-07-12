/*
 * Copyright 2018 Intel Corporation
 *
 * Licensed under the Apache License, Version 2.0 (the "License");
 * you may not use this file except in compliance with the License.
 * You may obtain a copy of the License at
 *
 *     http://www.apache.org/licenses/LICENSE-2.0
 *
 * Unless required by applicable law or agreed to in writing, software
 * distributed under the License is distributed on an "AS IS" BASIS,
 * WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
 * See the License for the specific language governing permissions and
 * limitations under the License.
 * ------------------------------------------------------------------------------
 */

use std::collections::HashMap;

use block::Block;

#[derive(Debug)]
pub enum BlockStoreError {
    Error(String),
    UnknownBlock,
}

pub trait BlockStore {
    fn get<'a>(&'a self, block_ids: Vec<String>) -> Box<Iterator<Item = &'a Block> + 'a>;

    fn delete(&mut self, block_ids: Vec<String>) -> Result<Vec<Block>, BlockStoreError>;

    fn put(&mut self, blocks: Vec<Block>) -> Result<(), BlockStoreError>;

    fn iter<'a>(&'a self) -> Box<Iterator<Item = &'a Block> + 'a>;
}

#[derive(Default)]
pub struct InMemoryBlockStore {
    block_by_block_id: HashMap<String, Block>,
    chain_head_num: u64,
    chain_head_id: String,
}

impl InMemoryBlockStore {
    pub fn new() -> Self {
        InMemoryBlockStore::default()
    }

    fn get_block_by_block_id(&self, block_id: &str) -> Option<&Block> {
        self.block_by_block_id.get(block_id)
    }
}

impl BlockStore for InMemoryBlockStore {
    fn get<'a>(&'a self, block_ids: Vec<String>) -> Box<Iterator<Item = &'a Block> + 'a> {
        let iterator: InMemoryGetBlockIterator = InMemoryGetBlockIterator::new(self, block_ids);

        Box::new(iterator)
    }

    fn delete(&mut self, block_ids: Vec<String>) -> Result<Vec<Block>, BlockStoreError> {
        if block_ids
            .iter()
            .any(|block_id| !self.block_by_block_id.contains_key(block_id))
        {
            return Err(BlockStoreError::UnknownBlock);
        }
        let blocks = block_ids.iter().map(|block_id| {
            let block = self.block_by_block_id
                .remove(block_id)
                .expect("Block removed during middle of delete operation");
            if block.block_num <= self.chain_head_num {
                self.chain_head_id = block.previous_block_id.clone();
                self.chain_head_num = block.block_num - 1;
            }
            block
        });

        Ok(blocks.collect())
    }

    fn put(&mut self, blocks: Vec<Block>) -> Result<(), BlockStoreError> {
        blocks.into_iter().for_each(|block| {
            if block.block_num > self.chain_head_num {
                self.chain_head_id = block.header_signature.clone();
                self.chain_head_num = block.block_num;
            }

            self.block_by_block_id
                .insert(block.header_signature.clone(), block);
        });
        Ok(())
    }

    fn iter<'a>(&'a self) -> Box<Iterator<Item = &'a Block> + 'a> {
        Box::new(InMemoryIter {
            blockstore: self,
            head: &self.chain_head_id,
        })
    }
}

struct InMemoryGetBlockIterator<'a> {
    blockstore: &'a InMemoryBlockStore,
    block_ids: Vec<String>,
    index: usize,
}

impl<'a> InMemoryGetBlockIterator<'a> {
    fn new(
        blockstore: &'a InMemoryBlockStore,
        block_ids: Vec<String>,
    ) -> InMemoryGetBlockIterator<'a> {
        InMemoryGetBlockIterator {
            blockstore,
            block_ids,
            index: 0,
        }
    }
}

impl<'a> Iterator for InMemoryGetBlockIterator<'a> {
    type Item = &'a Block;

    fn next(&mut self) -> Option<Self::Item> {
        let block = match self.block_ids.get(self.index) {
            Some(block_id) => self.blockstore.get_block_by_block_id(block_id),
            None => None,
        };
        self.index += 1;
        block
    }
}

struct InMemoryIter<'a> {
    blockstore: &'a InMemoryBlockStore,
    head: &'a str,
}

impl<'a> Iterator for InMemoryIter<'a> {
    type Item = &'a Block;

    fn next(&mut self) -> Option<Self::Item> {
        let block = self.blockstore.get_block_by_block_id(self.head);
        if let Some(b) = block {
            self.head = &b.previous_block_id;
        }
        block
    }
}
