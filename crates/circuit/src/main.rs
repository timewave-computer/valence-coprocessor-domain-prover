#![no_main]

use msgpacker::Unpackable as _;
use sp1_zkvm::lib::verify::verify_sp1_proof;
use valence_coprocessor::Hasher as _;
use valence_coprocessor_domain_prover::{Circuit, CircuitInput};
use valence_coprocessor_sp1::Sp1Hasher;
use zerocopy::FromBytes;

sp1_zkvm::entrypoint!(main);

pub fn main() {
    let input = sp1_zkvm::io::read_vec();
    let input = CircuitInput::unpack(&input).unwrap().1;

    if input.updates.is_empty() {
        let initial_root = [
            0x22, 0xfc, 0x64, 0x5f, 0xa0, 0x5c, 0x14, 0x0a, 0x7b, 0x33, 0x34, 0x5c, 0x1f, 0xf6,
            0xde, 0xb3, 0xf8, 0x1a, 0xfb, 0xb5, 0x75, 0x55, 0x44, 0x66, 0x61, 0xe4, 0xec, 0x74,
            0x09, 0x9f, 0xe0, 0xe4,
        ];
        let output = [&initial_root, input.vk.as_slice()].concat();
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
