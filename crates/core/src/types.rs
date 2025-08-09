use alloc::{string::String, vec::Vec};
use msgpacker::MsgPacker;
use serde::{Deserialize, Serialize};
use valence_coprocessor::{Hash, HistoricalTransitionProof};

/// An elected domain for verification.
#[derive(
    Debug, Default, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize, MsgPacker,
)]
pub struct Domain {
    pub id: Hash,
    pub vk: String,
}

/// A circuit definition.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize, MsgPacker)]
pub struct Circuit {
    pub initial_root: Hash,
    pub domains: Vec<Domain>,
}

impl Default for Circuit {
    fn default() -> Self {
        let domains = include_bytes!("../../../elf/domains.json");
        let domains = serde_json::from_slice(&domains[..]).unwrap();

        Self {
            initial_root: Hash::default(),
            domains,
        }
    }
}

/// The input of a circuit execution.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize, MsgPacker)]
pub struct CircuitInput {
    pub vk: Vec<u8>,
    pub updates: Vec<HistoricalTransitionProof>,
}

impl Default for CircuitInput {
    fn default() -> Self {
        Self {
            vk: include_bytes!("../../../elf/circuit-vkh32.bin").to_vec(),
            updates: Default::default(),
        }
    }
}

impl CircuitInput {
    pub fn initial_root(&self) -> Hash {
        self.updates
            .first()
            .map(|u| u.update.previous)
            .unwrap_or_default()
    }
}
