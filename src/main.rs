use clap::{Parser, Subcommand};
use eyre::eyre;
use obws::Client;
use std::{future::Future, pin::Pin, time::Duration};
use tracing::{debug, error, event, info, Level};

type Result<T> = std::result::Result<T, color_eyre::Report>;

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
    client.recording().start().await?;
    info!("recording");
    Ok(())
}

async fn stop(client: &Client) -> Result<()> {
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

    let for_duration = Duration::from_secs(2);

    match args.command {
        Command::Record => {
            event!(Level::INFO, "starting...");
            let now = std::time::Instant::now();
            loop {
                if now.elapsed() >= for_duration {
                    break;
                }
                std::thread::sleep(Duration::from_millis(100));
                if record(&client).await.is_ok() {
                    return Ok(());
                }
            }
            error!("failed to issue 'record' command");
        }
        Command::Stop => {
            info!("stopping...");
            let now = std::time::Instant::now();
            loop {
                if now.elapsed() >= for_duration {
                    break;
                }
                std::thread::sleep(Duration::from_millis(100));
                if stop(&client).await.is_ok() {
                    return Ok(());
                }
            }
            error!("failed to issue 'stop' command");
        }
    }

    Ok(())
}
