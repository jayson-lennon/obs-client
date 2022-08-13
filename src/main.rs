use anyhow::Result;
use clap::{Parser, Subcommand};
use obws::Client;
use tracing::{event, info, Level};

const LOGSPEC: [&str; 1] = ["obs_client"];

#[derive(Debug, Parser)]
#[clap(author, version, about)]
#[clap(propagate_version = true)]
struct Args {
    #[clap(subcommand)]
    command: Command,

    #[clap(env = "OBSWS_ADDR")]
    address: String,

    #[clap(env = "OBSWS_PORT")]
    port: u16,

    /// Password to connect to OBS
    #[clap(short, long, env = "OBSWS_PASSWORD")]
    password: Option<String>,
}

#[derive(Debug, Subcommand)]
enum Command {
    /// Start recording
    Record,
    /// Stop recording
    Stop,
}

async fn record(client: &Client) -> Result<()> {
    event!(Level::INFO, "starting...");
    client.recording().start().await?;
    info!("recording");
    Ok(())
}

async fn stop(client: &Client) -> Result<()> {
    info!("stopping...");
    client.recording().stop().await?;
    info!("stopped recording");
    Ok(())
}

fn install_tracing<S: AsRef<str>>(logspec: S) {
    use tracing_error::ErrorLayer;
    use tracing_subscriber::prelude::*;
    use tracing_subscriber::{fmt, EnvFilter};

    let fmt_layer = fmt::layer().with_target(false);
    let filter_layer = EnvFilter::try_from_default_env()
        .or_else(|_| EnvFilter::try_new(logspec.as_ref()))
        .unwrap();

    tracing_subscriber::registry()
        .with(filter_layer)
        .with(fmt_layer)
        .with(ErrorLayer::default())
        .init();
}

#[tokio::main]
async fn main() -> Result<()> {
    use dotenv::dotenv;
    dotenv().ok();

    install_tracing(LOGSPEC[0]);

    let (panic_hook, eyre_hook) = color_eyre::config::HookBuilder::default().into_hooks();

    eyre_hook.install()?;

    std::panic::set_hook(Box::new(move |pi| {
        tracing::error!("{}", panic_hook.panic_report(pi));
    }));

    let args = Args::parse();

    // Connect to the OBS instance through obs-websocket.
    let client = Client::connect(args.address, args.port, args.password).await?;

    match args.command {
        Command::Record => record(&client).await?,
        Command::Stop => stop(&client).await?,
    }

    Ok(())
}
