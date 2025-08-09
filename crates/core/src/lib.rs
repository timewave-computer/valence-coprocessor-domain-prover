#![no_std]

extern crate alloc;

use alloc::vec::Vec;
use msgpacker::{Packable as _, Unpackable as _};
use sp1_verifier::{Groth16Verifier, GROTH16_VK_BYTES};
use valence_coprocessor::{Hash, Hasher, HistoricalTransitionProof, Proof, ValidatedBlock};

mod state;
mod types;

pub use state::*;
pub use types::*;

impl Circuit {
    pub fn root<H: Hasher>(&self, updates: Vec<HistoricalTransitionProof>) -> anyhow::Result<Hash> {
        let mut root = updates
            .first()
            .map(|u| u.update.previous)
            .unwrap_or_default();

        for proof in updates {
            let update = proof.verify::<H>()?;

            anyhow::ensure!(root == update.previous, "unexpected root");

            root = update.root;

            let id = self
                .domains
                .iter()
                .enumerate()
                .find_map(|(i, d)| (d.id == update.block.domain).then_some(i));

            // won't verify lightclient proof if domain not elected
            let id = match id {
                Some(id) => id,
                None => continue,
            };

            let pi = ValidatedBlock {
                number: update.block.number,
                root: update.block.root,
                payload: Vec::new(),
            }
            .pack_to_vec();

            let proof = Proof::unpack(&update.block.payload)?.1;
            let proof = proof.decode()?.0;

            Groth16Verifier::verify(&proof, &pi, &self.domains[id].vk, &GROTH16_VK_BYTES)?;
        }

        Ok(root)
    }
}

#[test]
fn circuit_root_works() {
    use valence_coprocessor_sp1::Sp1Hasher;

    let input = include_bytes!("../../../assets/input.json");
    let CircuitInput { updates, .. } = serde_json::from_slice(input).unwrap();
    let circuit = Circuit::default();

    circuit.root::<Sp1Hasher>(updates).unwrap();
}
