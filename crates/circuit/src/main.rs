#![no_main]

use msgpacker::Unpackable as _;
use sp1_zkvm::lib::verify::verify_sp1_proof;
use valence_coprocessor::{Hash, Hasher as _};
use valence_coprocessor_domain_prover::{Circuit, CircuitInput};
use valence_coprocessor_sp1::Sp1Hasher;
use zerocopy::FromBytes;

sp1_zkvm::entrypoint!(main);

pub fn main() {
    let input = sp1_zkvm::io::read_vec();
    let input = CircuitInput::unpack(&input).unwrap().1;

    if input.updates.is_empty() {
        let output = [&Hash::default()[..], input.vk.as_slice()].concat();
        sp1_zkvm::io::commit_slice(&output);
        return;
    }

    let circuit = Circuit::default();

    let root = input.initial_root();
    let vkh = <[u32; 8]>::ref_from_bytes(&input.vk).unwrap();

    let digest = [&root[..], input.vk.as_slice()].concat();
    let digest = Sp1Hasher::hash_raw(&digest);

    verify_sp1_proof(&vkh, &digest);

    let root = circuit.root::<Sp1Hasher>(input.updates).unwrap();
    let output = [&root[..], input.vk.as_slice()].concat();

    sp1_zkvm::io::commit_slice(&output);
}
