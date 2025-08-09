use core::{cmp, ops::Bound};

use alloc::collections::btree_map::BTreeMap;
use msgpacker::MsgPacker;
use serde::{Deserialize, Serialize};
use valence_coprocessor::{Hash, HistoricalUpdate, Proof};

/// A controller state.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, MsgPacker)]
pub struct State {
    /// The co-processor historical update
    pub update: HistoricalUpdate,

    /// The latest computed proof.
    pub proof: Proof,

    /// The latest computed wrapper.
    pub wrapper: Proof,
}

impl PartialOrd for State {
    fn partial_cmp(&self, other: &Self) -> Option<cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for State {
    fn cmp(&self, other: &Self) -> cmp::Ordering {
        self.update.uuid.cmp(&other.update.uuid)
    }
}

impl State {
    /// Controller ID
    pub const ID: const_hex::Buffer<32> = {
        let mut id = [0u8; 32];

        id.copy_from_slice(include_bytes!("../../../elf/id.bin"));

        const_hex::const_encode(&id)
    };

    /// Returns `true` if the current state is older than `other`.
    pub fn is_older_than(&self, other: &Self) -> bool {
        self < other
    }

    /// Returns the root of the update.
    pub fn root(&self) -> anyhow::Result<Hash> {
        let inputs = self.wrapper.decode()?.1;

        Ok(Hash::try_from(&inputs[..32])?)
    }
}

#[derive(
    Debug, Default, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize, MsgPacker,
)]
pub struct ServiceState {
    items: BTreeMap<[u8; 16], State>,
    capacity: usize,
}

impl ServiceState {
    pub fn with_capacity(mut self, capacity: usize) -> Self {
        self.capacity = capacity.max(1);
        self
    }

    pub fn insert(&mut self, state: State) {
        if let Some(lower) = self.items.iter().next() {
            if &state < lower.1 && self.items.len() >= self.capacity {
                return;
            }
        }

        while self.items.len() >= self.capacity {
            self.items.pop_first();
        }

        self.items.insert(state.update.uuid, state);
    }

    pub fn get_lower_bound(&self, uuid: [u8; 16]) -> Option<&State> {
        let lower: Bound<[u8; 16]> = Bound::Unbounded;
        let upper = Bound::Included(uuid);

        self.items.range((lower, upper)).next_back().map(|(_, s)| s)
    }

    pub fn latest(&self) -> Option<&State> {
        self.items.iter().next_back().map(|(_, s)| s)
    }
}
