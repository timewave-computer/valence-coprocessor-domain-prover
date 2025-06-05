#![no_std]

extern crate alloc;

use alloc::vec::Vec;
use valence_coprocessor::Witness;

pub fn circuit(witnesses: Vec<Witness>) -> Vec<u8> {
    assert!(witnesses.is_empty());

    Vec::new()
}
