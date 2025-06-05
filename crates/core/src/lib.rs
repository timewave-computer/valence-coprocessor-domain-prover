use msgpacker::MsgPacker;

use serde_json::Value;
pub use valence_coprocessor;
pub use valence_coprocessor_client;
use valence_coprocessor_client::Client;

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, MsgPacker)]
pub struct Proof {
    pub program: valence_coprocessor::Proof,
    pub domain: valence_coprocessor::Proof,
}

// pub async fn prove<C: AsRef<str>>(&self, circuit: C, args: &Value) -> anyhow::Result<Proof> {
impl Proof {
    /// The deployed domain prover circuit id.
    pub const DOMAIN_CIRCUIT: &str =
        "520d2b2cc9a7e4005c10ae62e14745a737459e1e39f387e23a6607d567b2b87d";

    pub async fn prove<C: AsRef<str>>(
        client: &Client,
        circuit: C,
        args: &Value,
    ) -> anyhow::Result<Self> {
        let circuit = circuit.as_ref();
        let program = client.prove(circuit, args).await?;

        let root = program.decode()?.1;
        let root = hex::encode(&root[..32]);

        let domain = client
            .prove_with_root(Self::DOMAIN_CIRCUIT, root, &Default::default())
            .await?;

        Ok(Self { program, domain })
    }
}

#[cfg(test)]
mod tests {
    use serde_json::json;
    use valence_coprocessor_client::Client;

    use super::Proof;

    #[tokio::test]
    async fn controller_works() {
        let witnesses = Client::default()
            .get_witnesses(Proof::DOMAIN_CIRCUIT, &Default::default())
            .await
            .unwrap();

        assert!(witnesses.is_empty());
    }

    #[tokio::test]
    async fn prove_works() {
        let circuit = "7e0207a1fa0a979282b7246c028a6a87c25bc60f7b6d5230e943003634e897fd";
        let args = json!({"value": 42});
        let client = Client::default();

        let proof = Proof::prove(&client, circuit, &args).await.unwrap();
        let program = proof.program.decode().unwrap().1;
        let domain = proof.domain.decode().unwrap().1;

        assert_eq!(&program[..32], &domain[..32]);
    }
}
