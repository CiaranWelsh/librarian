//! Server-side TOML config. The MCP server is a thin client of the query
//! daemon, so it only needs to know which collection to query and where the
//! daemon lives — no qdrant/manifest/embedder configuration here.

use serde::Deserialize;

#[derive(Debug, Deserialize)]
pub struct Config {
    pub collection: String,
    pub daemon_url: String,
}

pub fn load(config_path: &std::path::Path) -> Result<Config, String> {
    let s = std::fs::read_to_string(config_path).map_err(|e| format!("config io: {e}"))?;
    toml::from_str(&s).map_err(|e| format!("config parse: {e}"))
}
