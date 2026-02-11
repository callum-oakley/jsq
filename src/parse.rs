use anyhow::{Context, Result};
use indexmap::IndexMap;
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

/// Parse CSV in to a JSON string.
pub fn csv(s: &str) -> Result<String> {
    let mut rows: Vec<IndexMap<&str, Value>> = Vec::new();
    let mut reader = csv::Reader::from_reader(s.as_bytes());
    let headers = reader.headers()?;
    for record in csv::Reader::from_reader(s.as_bytes()).records() {
        rows.push(
            headers
                .iter()
                .zip(record?.iter().map(|s| match serde_json::from_str(s) {
                    // Avoid parsing strings, since this will have the effect of stripping the outer
                    // quotes...
                    Err(_) | Ok(Value::String(_)) => Value::String(s.to_string()),
                    // ...but it's convenient to parse anything else that looks like JSON.
                    Ok(value) => value,
                }))
                .collect(),
        );
    }
    Ok(serde_json::to_string(&rows)?)
}
