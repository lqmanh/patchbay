use std::path::Path;

use serde::Serialize;

pub fn to_pretty_json(value: &impl Serialize) -> anyhow::Result<String> {
    let mut text = serde_json::to_string_pretty(value)?;
    text.push('\n');
    Ok(text)
}

pub fn write_json(path: &Path, value: &impl Serialize) -> anyhow::Result<()> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    std::fs::write(path, to_pretty_json(value)?)?;
    Ok(())
}
