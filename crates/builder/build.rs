use std::{env, fs, path::PathBuf, process::Command};

use sp1_sdk::{HashableKey as _, Prover as _, ProverClient};
use valence_coprocessor::{ControllerData, DomainData};
use zerocopy::IntoBytes as _;

fn main() {
    println!("cargo:rerun-if-env-changed=VALENCE_REBUILD");

    if env::var("VALENCE_REBUILD").is_err() {
        return;
    }

    let manifest = env::var("CARGO_MANIFEST_DIR").unwrap();
    let manifest = PathBuf::from(manifest).parent().unwrap().to_path_buf();
    let root = manifest.parent().unwrap();
    let out = root.join("elf");
    let circuits = root
        .join("target")
        .join("elf-compilation")
        .join("riscv32im-succinct-zkvm-elf")
        .join("release");

    let mut wrapper = fs::read(out.join("wrapper.bin")).unwrap();

    if env::var("VALENCE_REBUILD_SKIP_CIRCUIT").is_err() {
        // circuit
        // elected domains

        let prover = ProverClient::builder().cpu().build();

        // - ethereum

        let id = DomainData::identifier_from_parts("ethereum-electra-alpha");

        // TODO fetch from repo or coprocessor
        let vk = "0x0024f93469cf0354fbb33989ddd1a3c8b5d62b2e8bb6fa20a4bc5325269683c0";
        let domains = serde_json::json!([{
            "id": id,
            "vk": vk,
        }]);
        let domains = serde_json::to_string(&domains).unwrap();

        fs::write(out.join("domains.json"), domains).unwrap();

        // inner circuit

        sp1_build::build_program("../circuit");

        let circuit = circuits.join("valence-coprocessor-domain-prover-circuit");
        let elf = fs::read(&circuit).unwrap();

        let (_, vk) = prover.setup(&elf);
        let vkb = vk.bytes32();
        let vkh = vk.vk.hash_u32();
        let vk = serde_cbor::to_vec(&vk).unwrap();

        fs::write(out.join("circuit.bin"), elf).unwrap();
        fs::write(out.join("circuit-vk.bin"), vk).unwrap();
        fs::write(out.join("circuit-bytes32"), vkb).unwrap();
        fs::write(out.join("circuit-vkh32.bin"), vkh.as_bytes()).unwrap();

        // wrapper circuit

        sp1_build::build_program("../wrapper");

        wrapper = fs::read(&circuits.join("valence-coprocessor-domain-prover-wrapper")).unwrap();

        let (_, vk) = prover.setup(&wrapper);
        let vkb = vk.bytes32();
        let vkh = vk.vk.hash_u32();
        let vk = serde_cbor::to_vec(&vk).unwrap();

        fs::write(out.join("wrapper.bin"), &wrapper).unwrap();
        fs::write(out.join("wrapper-vk.bin"), vk).unwrap();
        fs::write(out.join("wrapper-bytes32"), vkb).unwrap();
        fs::write(out.join("wrapper-vkh32.bin"), vkh.as_bytes()).unwrap();
    }

    // controller

    assert!(Command::new("cargo")
        .current_dir(&root)
        .args([
            "build",
            "-p",
            "valence-coprocessor-domain-prover-controller",
            "--target",
            "wasm32-unknown-unknown",
            "--release",
        ])
        .status()
        .unwrap()
        .success());

    let wasm = root
        .join("target")
        .join("wasm32-unknown-unknown")
        .join("release")
        .join("valence_coprocessor_domain_prover_controller.wasm");

    let wasm = fs::read(&wasm).unwrap();

    fs::write(out.join("controller.wasm"), &wasm).unwrap();

    // id

    let id = ControllerData::default()
        .with_controller(wasm)
        .with_circuit(wrapper)
        .identifier();

    fs::write(out.join("id.bin"), &id).unwrap();
    fs::write(out.join("id.txt"), hex::encode(&id)).unwrap();
}
