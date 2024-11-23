use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::path::Path;

#[derive(Debug, Serialize, Deserialize)]
pub struct Config {
    pub output_folder: String,
    pub ontologies: Vec<String>,
}

impl Config {
    pub fn from_file<P: AsRef<Path>>(path: P) -> Result<Self> {
        let content = std::fs::read_to_string(path)?;
        let config = serde_json::from_str(&content)?;
        Ok(config)
    }
}
