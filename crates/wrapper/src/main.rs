#![no_main]

use sha2_v0_10_8::{Digest as _, Sha256};
use sp1_zkvm::lib::verify::verify_sp1_proof;
use zerocopy::FromBytes;

sp1_zkvm::entrypoint!(main);

pub fn main() {
    let vk = include_bytes!("../../../elf/circuit-vkh32.bin");
    let inputs = sp1_zkvm::io::read_vec();

    let root = &inputs[..32];
    let vk_p = &inputs[32..];

    assert_eq!(vk_p, vk);

    let digest = Sha256::digest(&inputs).into();
    let vk = <[u32; 8]>::ref_from_bytes(&vk[..]).unwrap();

    verify_sp1_proof(&vk, &digest);

    sp1_zkvm::io::commit_slice(&root);
}
