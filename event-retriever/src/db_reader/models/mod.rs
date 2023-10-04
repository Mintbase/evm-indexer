use ethers::types::{Address, U256};
use std::{cmp::Ordering, collections::BinaryHeap, fmt::Debug};

mod conversions;
pub(crate) mod db;

#[derive(Debug)]
pub struct NftEvent {
    pub base: EventBase,
    pub meta: EventMeta,
}

impl Eq for NftEvent {}

impl PartialEq<Self> for NftEvent {
    fn eq(&self, other: &Self) -> bool {
        self.base == other.base
    }
}

impl PartialOrd for NftEvent {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for NftEvent {
    fn cmp(&self, other: &Self) -> Ordering {
        self.base.cmp(&other.base)
    }
}
#[derive(Debug, PartialEq)]
pub enum EventMeta {
    ApprovalForAll(ApprovalForAll),
    Erc1155TransferBatch(Erc1155TransferBatch),
    Erc1155TransferSingle(Erc1155TransferSingle),
    Erc1155Uri(Erc1155Uri),
    Erc721Approval(Erc721Approval),
    Erc721Transfer(Erc721Transfer),
}

/// Every Ethereum Event emits these properties
#[derive(Debug)]
pub struct EventBase {
    pub block_number: u64,
    pub log_index: u64,
    pub transaction_index: u64,
    pub contract_address: Address,
}

impl Eq for EventBase {}

impl PartialEq<Self> for EventBase {
    fn eq(&self, other: &Self) -> bool {
        self.block_number == other.block_number && self.log_index == other.log_index
    }
}

impl PartialOrd for EventBase {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for EventBase {
    fn cmp(&self, other: &Self) -> Ordering {
        self.block_number
            .cmp(&other.block_number)
            .then_with(|| self.log_index.cmp(&other.log_index))
    }
}
#[derive(Debug, PartialEq)]
pub struct ApprovalForAll {
    pub owner: Address,
    pub operator: Address,
    pub approved: bool,
}

#[derive(Debug, PartialEq)]
pub struct Erc1155TransferBatch {
    pub operator: Address,
    pub from: Address,
    pub to: Address,
    pub ids: Vec<U256>,
    pub values: Vec<U256>,
}

#[derive(Debug, PartialEq)]
pub struct Erc1155TransferSingle {
    pub operator: Address,
    pub from: Address,
    pub to: Address,
    pub id: U256,
    pub value: U256,
}

#[derive(Debug, PartialEq)]
pub struct Erc1155Uri {
    pub id: U256,
    pub value: String,
}

#[derive(Debug, PartialEq)]
pub struct Erc721Approval {
    pub owner: Address,
    pub approved: Address,
    pub id: U256,
}

#[derive(Debug, PartialEq)]
pub struct Erc721Transfer {
    pub from: Address,
    pub to: Address,
    pub token_id: U256,
}

/// Merges a collection of Sorted Iterators into a sorted Vector of the Item.
/// This implementation makes use of a Min Heap.
pub fn merge_sorted_iters<T: Ord>(mut iters: Vec<Box<dyn Iterator<Item = T>>>) -> Vec<T> {
    use std::cmp::Reverse;
    let mut heap = BinaryHeap::new();

    // Initialize heap with the first item from each iterator
    for (index, iter) in iters.iter_mut().enumerate() {
        if let Some(item) = iter.next() {
            heap.push(Reverse((item, index)));
        }
    }

    let mut result = Vec::new();
    while let Some(Reverse((item, index))) = heap.pop() {
        let iter = &mut iters[index];
        for next_item in iter.by_ref() {
            if next_item <= item {
                // All duplicates get pushed to results.
                result.push(next_item);
            } else {
                // Push the next item from the iterator back to the heap
                heap.push(Reverse((next_item, index)));
                break;
            }
        }
        result.push(item);
    }

    result
}

#[cfg(test)]
mod tests {
    use super::*;

    fn event_base_for_block_log_index(block_number: u64, log_index: u64) -> EventBase {
        EventBase {
            block_number,
            log_index,
            transaction_index: 0,
            contract_address: Default::default(),
        }
    }
    #[test]
    fn event_base_comparison() {
        let eb_0_0 = event_base_for_block_log_index(0, 0);
        let eb_0_1 = event_base_for_block_log_index(0, 1);
        let eb_1_0 = event_base_for_block_log_index(1, 0);
        let eb_1_1 = event_base_for_block_log_index(1, 1);
        assert!(eb_0_0 < eb_0_1);
        assert!(eb_0_0 < eb_1_0);
        assert!(eb_0_0 < eb_1_1);
        assert!(eb_0_1 < eb_1_0);
        assert!(eb_0_1 < eb_1_1);
        assert!(eb_1_0 < eb_1_1);

        assert_eq!(
            eb_0_0,
            EventBase {
                block_number: 0,
                log_index: 0,
                // These are different, but the event equality is defined entirely on primary key!
                transaction_index: 1,
                contract_address: Address::from([1u8; 20]),
            }
        );
    }

    #[test]
    fn merge_sorted() {
        assert_eq!(
            merge_sorted_iters(vec![
                Box::new([1, 3, 5, 7].into_iter()),
                Box::new([2, 2, 4, 6].into_iter())
            ]),
            [1, 2, 2, 3, 4, 5, 6, 7]
        );
        assert_eq!(
            merge_sorted_iters(vec![
                Box::new(vec![1, 3, 5, 7].into_iter()),
                Box::new(vec![2, 2, 4, 6].into_iter()),
                Box::new(vec![1, 3, 4, 5, 6, 7].into_iter())
            ]),
            [1, 1, 2, 2, 3, 3, 4, 4, 5, 5, 6, 6, 7, 7]
        );
    }
}
