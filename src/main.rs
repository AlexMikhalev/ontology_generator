use anyhow::Result;
use atomic_lib::{Store, Storelike};
use clap::Parser;
use std::path::PathBuf;

mod config;
mod generator;

use config::Config;
use generator::OntologyGenerator;

#[derive(Parser)]
#[command(about = "Generate Rust types from Atomic Data ontologies")]
struct Cli {
    #[arg(short, long, default_value = "atomic.config.json")]
    config: PathBuf,

    #[arg(short, long)]
    agent: Option<String>,
}

fn main() -> Result<()> {
    let cli = Cli::parse();

    // Initialize store
    let mut store = Store::init()?;
    store.populate()?;

    // Set up agent if provided
    if let Some(agent_secret) = cli.agent {
        let agent = store.create_agent(Some(&agent_secret))?;
        store.set_default_agent(agent);
    }

    // Load config
    let config = Config::from_file(&cli.config)?;

    // Create and run generator
    let generator = OntologyGenerator::new(store, config)?;
    generator.generate()?;

    println!("Successfully generated Rust types from ontologies!");
    Ok(())
}
