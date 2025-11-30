use anyhow::{Context, Result};
use serde_json::Value;

/// Parse JSON in to a JSON string.
pub fn json(s: &str) -> Result<String> {
    Ok(serde_json::from_str::<Value>(s)
        .context("parsing JSON")?
        .to_string())
}

/// Parse YAML in to a JSON string.
pub fn yaml(s: &str) -> Result<String> {
    Ok(serde_yaml::from_str::<Value>(s)
        .context("parsing YAML")?
        .to_string())
}

/// Parse TOML in to a JSON string.
pub fn toml(s: &str) -> Result<String> {
    Ok(toml::from_str::<Value>(s)
        .context("parsing TOML")?
        .to_string())
}

/// Parse JSON5 in to a JSON string.
pub fn json5(s: &str) -> Result<String> {
    Ok(json5::from_str::<Value>(s)
        .context("parsing JSON5")?
        .to_string())
}
