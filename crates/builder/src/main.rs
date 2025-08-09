use clap::{Parser, Subcommand};
use tracing_subscriber::{fmt, layer::SubscriberExt as _, util::SubscriberInitExt as _, EnvFilter};
use valence_coprocessor_client::Client;

#[derive(Parser)]
struct Cli {
    /// Socket to the co-processor service.
    #[arg(
        long,
        value_name = "COPROCESSOR",
        default_value = "https://service.coprocessor.valence.zone"
    )]
    coprocessor: String,

    #[command(subcommand)]
    pub cmd: Commands,
}

#[derive(Debug, Subcommand)]
pub enum Commands {
    /// Deploys definitions to the co-processor
    Deploy,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let Cli { coprocessor, cmd } = Cli::parse();

    let filter_layer = EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info"));
    let fmt_layer = fmt::layer().with_target(false);

    tracing_subscriber::registry()
        .with(filter_layer)
        .with(fmt_layer)
        .init();

    tracing::info!("Executing command `{:?}` on {coprocessor}...", cmd);

    let coprocessor = Client::default().with_coprocessor(coprocessor);

    match cmd {
        Commands::Deploy => {
            let circuit = include_bytes!("../../../elf/wrapper.bin");
            let controller = include_bytes!("../../../elf/controller.wasm");
            let id = coprocessor
                .deploy_controller(controller, circuit, None)
                .await?;

            println!("{id}");
        }
    }

    Ok(())
}
