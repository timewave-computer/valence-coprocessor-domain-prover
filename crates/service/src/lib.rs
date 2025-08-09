use std::sync::Arc;

use msgpacker::Packable as _;
use sp1_sdk::SP1VerifyingKey;
use tokio::sync::Mutex;
use valence_coprocessor::{ControllerData, Hash, Proof};
use valence_coprocessor_client::Client as Coprocessor;
use valence_coprocessor_domain_prover::{CircuitInput, ServiceState, State};
use valence_coprocessor_prover::{
    client::Client as Prover,
    types::{ProofType, RecursiveProof},
};

pub const ID: &[u8] = include_bytes!("../../../elf/id.bin");
pub const INNER_ELF: &[u8] = include_bytes!("../../../elf/circuit.bin");
pub const INNER_VK: &[u8] = include_bytes!("../../../elf/circuit-vk.bin");
pub const INNER_VK_B32: &[u8] = include_bytes!("../../../elf/circuit-vkh32.bin");
pub const WRAPPER_ELF: &[u8] = include_bytes!("../../../elf/wrapper.bin");
pub const WRAPPER_VK: &[u8] = include_bytes!("../../../elf/wrapper-bytes32");

#[derive(Clone)]
pub struct App {
    service: Arc<Mutex<ServiceState>>,
    coprocessor: Coprocessor,
    prover: Prover,
    inner: SP1VerifyingKey,
    inner_hash: Hash,
    wrapper_hash: Hash,
    wrapper_vk: String,
    id: String,
}

impl App {
    pub fn new(capacity: usize) -> Self {
        let service = ServiceState::default().with_capacity(capacity);
        let service = Arc::new(Mutex::new(service));
        let coprocessor = Coprocessor::default();
        let prover = Prover::default();
        let inner = serde_cbor::from_slice(INNER_VK).unwrap();
        let inner_hash = ControllerData::identifier_from_parts(INNER_ELF, 0);
        let wrapper_hash = Hash::try_from(ID).unwrap();
        let wrapper_vk = String::from_utf8(WRAPPER_VK.to_vec()).unwrap();
        let id = State::ID.to_string();

        Self {
            service,
            coprocessor,
            prover,
            inner,
            inner_hash,
            wrapper_hash,
            wrapper_vk,
            id,
        }
    }

    pub fn with_coprocessor<C: AsRef<str>>(mut self, coprocessor: C) -> Self {
        self.coprocessor = self.coprocessor.with_coprocessor(coprocessor);
        self
    }

    pub fn with_prover<P: ToString>(mut self, prover: P) -> Self {
        self.prover = Prover::new(prover);
        self
    }

    pub fn id(&self) -> &str {
        &self.id
    }

    pub fn vk(&self) -> &str {
        &self.wrapper_vk
    }

    pub async fn init(self) -> anyhow::Result<Self> {
        tracing::info!("Loading controller `{}`...", self.id);

        let state = self.coprocessor.get_storage_raw(&self.id).await;

        tracing::info!("Data present on the co-processor: {}...", state.is_ok());

        let state: Option<State> = state.ok().and_then(|s| serde_json::from_slice(&s).ok());

        tracing::info!(
            "Co-processor data parsed into valid state: {}...",
            state.is_some()
        );

        let state = match state {
            Some(s) => {
                self.service.lock().await.insert(s.clone());
                s
            }
            None => {
                tracing::info!("Data not available; bootstrapping...");

                let input = CircuitInput::default().pack_to_vec();
                let proof = self.prover.get_sp1_proof(
                    self.inner_hash,
                    ProofType::Compressed,
                    &input,
                    &[],
                    |_| Ok(INNER_ELF.to_vec()),
                )?;

                self.publish_wrapper_proof(proof).await?
            }
        };

        tracing::info!("State `{}` loaded...", hex::encode(&state.update.root));

        Ok(self)
    }

    pub async fn latest(&self) -> Option<State> {
        self.service.lock().await.latest().cloned()
    }

    pub async fn insert_state(&self, proof: Proof, wrapper: Proof) -> anyhow::Result<State> {
        tracing::debug!("inserting new state...");

        let root = wrapper.decode()?.1;
        let root = Hash::try_from(root.as_slice())?;

        tracing::debug!("root computed...");

        let update = self.coprocessor.get_historical_update(&root).await?;
        let state = State {
            update,
            proof,
            wrapper,
        };

        tracing::debug!("new state computed...");

        let should_update = {
            let mut service = self.service.lock().await;

            service.insert(state.clone());
            service.latest().filter(|l| *l < &state).is_none()
        };

        if should_update {
            tracing::info!(
                "produced latest update `{}`; publishing...",
                hex::encode(state.update.root)
            );

            let bytes = serde_json::to_vec(&state)?;
            let updated = self.coprocessor.set_storage_raw(&self.id, bytes).await;

            match updated {
                Ok(true) => tracing::info!("co-processor updated."),
                Ok(false) => tracing::warn!("co-processor not updated."),
                Err(e) => tracing::warn!("co-processor not updated: {e}"),
            }
        }

        Ok(state)
    }

    pub async fn compute_inner_proof(&self, root: &Hash) -> anyhow::Result<Option<Proof>> {
        tracing::debug!("computing inner proof for `{}`...", hex::encode(root));

        let update = self.coprocessor.get_historical_update(&root).await?;
        let state = {
            self.service
                .lock()
                .await
                .get_lower_bound(update.uuid)
                .cloned()
                .ok_or_else(|| anyhow::anyhow!("failed to find the lower bound for the proof"))?
        };

        let state_root = state.root()?;

        tracing::debug!("lower bound state: `{}`...", hex::encode(state_root));

        let proof = state.proof.clone();
        let from = state_root;
        let to = *root;

        if to == from {
            tracing::debug!("cache hit.");

            return Ok(Some(proof));
        }

        tracing::debug!(
            "cache miss; fetching updates from `{}` to `{}`...",
            hex::encode(from),
            hex::encode(to)
        );

        let updates = self.coprocessor.get_historical_updates(&from, &to).await?;
        if updates.is_empty() {
            tracing::debug!("no updates available.");

            return Ok(None);
        }

        tracing::debug!("got `{}` updates, proving inner...", updates.len());

        let input = CircuitInput {
            vk: INNER_VK_B32.to_vec(),
            updates,
        }
        .pack_to_vec();

        let proof = RecursiveProof::try_from_compressed_proof(&state.proof, self.inner.vk.clone())?;

        let proof = self.prover.get_sp1_proof(
            self.inner_hash,
            ProofType::Compressed,
            &input,
            &[proof],
            |_| Ok(INNER_ELF.to_vec()),
        )?;

        tracing::debug!("inner proof computed.");

        Ok(Some(proof))
    }

    pub async fn publish_wrapper_proof(&self, proof: Proof) -> anyhow::Result<State> {
        let inputs = proof.decode()?.1;
        let recursive = RecursiveProof::try_from_compressed_proof(&proof, self.inner.vk.clone())?;

        let wrapper = self.prover.get_sp1_proof(
            self.wrapper_hash,
            ProofType::Groth16,
            &inputs,
            &[recursive],
            |_| Ok(WRAPPER_ELF.to_vec()),
        )?;

        tracing::debug!("computed wrapper proof; publishing...");

        self.insert_state(proof, wrapper).await
    }

    pub async fn update_to_latest(&self) -> anyhow::Result<Option<State>> {
        tracing::debug!("checking for recent historical root...");

        let root = self.coprocessor.get_historical().await?;
        let update = self.coprocessor.get_historical_update(&root).await?;

        tracing::debug!("got latest update `{}`...", hex::encode(update.root));

        match self.latest().await {
            Some(l) if update.uuid <= l.update.uuid => {
                tracing::debug!("already up-to-date; skipping...");
                return Ok(None);
            }
            None => {
                tracing::error!("failed to fetch latest update!");
            }
            _ => tracing::debug!("proceed with proof computation..."),
        }

        let proof = match self.compute_inner_proof(&root).await? {
            Some(p) => p,
            None => {
                tracing::debug!("no inner proof available; skipping...");
                return Ok(None);
            }
        };

        self.publish_wrapper_proof(proof).await.map(Some)
    }
}
