#![cfg_attr(feature = "wasm", no_std)]

use serde_json::Value;
use valence_coprocessor_wasm::abi;

extern crate alloc;

#[no_mangle]
pub extern "C" fn get_witnesses() {
    abi::ret(&Value::Null).ok();
}

#[no_mangle]
pub extern "C" fn entrypoint() {
    abi::ret(&Value::Null).ok();
}
