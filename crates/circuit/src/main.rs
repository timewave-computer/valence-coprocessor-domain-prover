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
            0xfd, 0xd3, 0x75, 0x61, 0x72, 0x3c, 0xa9, 0x2a, 0x70, 0x33, 0xae, 0xb5, 0x2d, 0xdc,
            0x02, 0x7d, 0x73, 0x98, 0x04, 0x2b, 0xa9, 0x3b, 0xe3, 0x16, 0xdd, 0x6f, 0x14, 0x14,
            0x83, 0x95, 0x26, 0x48,
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
