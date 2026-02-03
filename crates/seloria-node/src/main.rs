use std::path::PathBuf;

use anyhow::Result;
use clap::Parser;
use tracing::{error, info, Level};
use tracing_subscriber::FmtSubscriber;

mod cli;
mod config;
mod node;
mod txgen;

use cli::{Cli, Commands};
use config::{generate_sample_config, NodeConfig};
use node::Node;

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize logging
    let _subscriber = FmtSubscriber::builder()
        .with_max_level(Level::INFO)
        .with_target(false)
        .pretty()
        .init();

    let cli = Cli::parse();

    match cli.command {
        Commands::Run { config } => {
            run_node(config).await?;
        }
        Commands::Init { output, with_keys } => {
            init_config(output, with_keys)?;
        }
        Commands::Keygen { output } => {
            generate_keypair(output)?;
        }
        Commands::Status { endpoint } => {
            show_status(&endpoint).await?;
        }
        Commands::Tx { endpoint, file } => {
            submit_transaction(&endpoint, file).await?;
        }
        Commands::Txgen { command } => {
            txgen::handle_txgen(command)?;
        }
    }

    Ok(())
}

/// Run a Seloria node
async fn run_node(config_path: PathBuf) -> Result<()> {
    info!("Loading configuration from {:?}", config_path);

    let config = if config_path.exists() {
        NodeConfig::load(&config_path)?
    } else {
        error!(
            "Configuration file not found: {:?}. Run 'seloria init' to create one.",
            config_path
        );
        return Err(anyhow::anyhow!("Configuration file not found"));
    };

    let node = Node::new(config)?;
    node.run().await?;

    Ok(())
}

/// Initialize a new configuration file
fn init_config(output: PathBuf, _with_keys: bool) -> Result<()> {
    info!("Generating sample configuration");

    let config = generate_sample_config();
    config.save(&output)?;

    info!("Configuration saved to {:?}", output);
    info!("Generated keys:");
    info!("  Validator key: {}", config.validator_key.unwrap_or_default());

    println!("\nConfiguration file created: {}", output.display());
    println!("Edit the file to customize your node settings.");
    println!("\nTo start the node, run:");
    println!("  seloria run --config {}", output.display());

    Ok(())
}

/// Generate a new keypair
fn generate_keypair(output: Option<PathBuf>) -> Result<()> {
    let keypair = seloria_core::KeyPair::generate();

    println!("Generated new keypair:");
    println!("  Public key:  {}", keypair.public.to_hex());
    println!("  Secret key:  {}", keypair.secret.to_hex());

    if let Some(path) = output {
        std::fs::write(&path, keypair.secret.to_hex())?;
        info!("Secret key saved to {:?}", path);
    }

    println!("\nWARNING: Keep your secret key safe! Do not share it with anyone.");

    Ok(())
}

/// Show node status
async fn show_status(endpoint: &str) -> Result<()> {
    let url = format!("{}/status", endpoint);

    let response = reqwest::get(&url).await?;

    if response.status().is_success() {
        let status: serde_json::Value = response.json().await?;
        println!("Node Status:");
        println!("{}", serde_json::to_string_pretty(&status)?);
    } else {
        error!("Failed to get status: {}", response.status());
    }

    Ok(())
}

/// Submit a transaction
async fn submit_transaction(endpoint: &str, file: PathBuf) -> Result<()> {
    let content = std::fs::read_to_string(&file)?;
    let tx: seloria_core::Transaction = serde_json::from_str(&content)?;

    let url = format!("{}/tx", endpoint);

    let client = reqwest::Client::new();
    let response = client
        .post(&url)
        .json(&serde_json::json!({ "transaction": tx }))
        .send()
        .await?;

    if response.status().is_success() {
        let result: serde_json::Value = response.json().await?;
        println!("Transaction submitted:");
        println!("{}", serde_json::to_string_pretty(&result)?);
    } else {
        let error: serde_json::Value = response.json().await?;
        error!("Failed to submit transaction:");
        println!("{}", serde_json::to_string_pretty(&error)?);
    }

    Ok(())
}
