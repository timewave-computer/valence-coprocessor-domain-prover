use std::{net::SocketAddr, time::Duration};

use clap::Parser;
use poem::{http::StatusCode, listener::TcpListener, web::Data, EndpointExt as _, Route};
use poem_openapi::{payload::Json, OpenApi, OpenApiService};
use serde_json::{json, Value};
use tokio::time::sleep;
use tracing_subscriber::{fmt, layer::SubscriberExt as _, util::SubscriberInitExt as _, EnvFilter};
use valence_coprocessor_domain_prover_service::App;

#[derive(Parser)]
struct Cli {
    /// Bind to the provided socket
    #[arg(short, long, value_name = "SOCKET", default_value = "0.0.0.0:37279")]
    bind: SocketAddr,

    /// Address to the co-processor service backend.
    #[arg(
        short,
        long,
        value_name = "COPROCESSOR",
        default_value = "https://service.coprocessor.valence.zone"
    )]
    coprocessor: String,

    /// Address to the prover service backend.
    #[arg(
        short,
        long,
        value_name = "PROVER",
        default_value = "wss://prover.coprocessor.valence.zone"
    )]
    prover: String,

    /// Cache capacity
    #[arg(long, value_name = "CAPACITY", default_value_t = 1000)]
    capacity: usize,

    /// Update interval (ms)
    #[arg(long, value_name = "INTERVAL", default_value_t = 60000)]
    interval: u64,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let Cli {
        bind,
        coprocessor,
        prover,
        capacity,
        interval,
    } = Cli::parse();

    let filter_layer = EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info"));
    let fmt_layer = fmt::layer().with_target(false);

    tracing_subscriber::registry()
        .with(filter_layer)
        .with(fmt_layer)
        .init();

    tracing::info!("loading app...");

    let app = App::new(capacity)
        .with_coprocessor(coprocessor)
        .with_prover(prover)
        .init()
        .await?;

    let latest = app
        .latest()
        .await
        .ok_or_else(|| anyhow::anyhow!("failed to load initial state"))?
        .update
        .root;

    tracing::info!("app loaded with latest root `{}`...", hex::encode(latest));

    let app_spawn = app.clone();

    tokio::spawn(async move {
        let interval = Duration::from_millis(interval);

        loop {
            tracing::debug!("state update to latest...");

            if let Err(e) = app_spawn.update_to_latest().await {
                tracing::error!("error updating state: {e}");
            }

            sleep(interval).await;
        }
    });

    let api_service = OpenApiService::new(Api, env!("CARGO_PKG_NAME"), env!("CARGO_PKG_VERSION"))
        .server(format!("http://{}/api", &bind));

    let app = Route::new()
        .nest("/", api_service.swagger_ui())
        .nest("/spec", api_service.spec_endpoint())
        .nest("/spec/yaml", api_service.spec_endpoint_yaml())
        .nest("/api", api_service)
        .data(app);

    tracing::info!("API loaded, listening on `{}`...", &bind);

    poem::Server::new(TcpListener::bind(&bind)).run(app).await?;

    Ok(())
}

pub struct Api;

#[OpenApi]
impl Api {
    /// Returns the latest domain proof.
    #[oai(path = "/latest", method = "get")]
    pub async fn latest(&self, app: Data<&App>) -> poem::Result<Json<Value>> {
        let state = app
            .latest()
            .await
            .and_then(|s| serde_json::to_value(s).ok())
            .ok_or_else(|| {
                tracing::error!("failed to fetch latest state!");

                poem::Error::from_status(StatusCode::INTERNAL_SERVER_ERROR)
            })?;

        Ok(Json(state))
    }

    /// Returns the domain proof constants.
    #[oai(path = "/consts", method = "get")]
    pub async fn consts(&self, app: Data<&App>) -> poem::Result<Json<Value>> {
        Ok(Json(json!({
            "id": app.id(),
            "vk": app.vk(),
        })))
    }
}
