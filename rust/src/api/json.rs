use serde::Serialize;
use serde::de::DeserializeOwned;
use std::fs;
use std::path::Path;

/// Reads and parses a JSON file into the specified type.
pub fn read<T: DeserializeOwned>(path: impl AsRef<Path>) -> Result<T, String> {
    let content = fs::read_to_string(path).map_err(|e| e.to_string())?;
    serde_json::from_str(&content).map_err(|e| e.to_string())
}

/// Serializes and writes a value to a JSON file.
pub fn write<T: Serialize>(path: impl AsRef<Path>, value: &T) -> Result<(), String> {
    let content = serde_json::to_string_pretty(value).map_err(|e| e.to_string())?;
    fs::write(path, content).map_err(|e| e.to_string())
}

/// Serializes and writes a value to a JSON file atomically (write to .tmp then rename).
pub fn write_atomic<T: Serialize>(path: impl AsRef<Path>, value: &T) -> Result<(), String> {
    let path = path.as_ref();
    let temp_path = path.with_extension("tmp");
    write(temp_path, value)?;
    fs::rename(path.with_extension("tmp"), path).map_err(|e| e.to_string())
}

/// Parses a JSON string into the specified type.
pub fn parse<T: DeserializeOwned>(json: &str) -> Result<T, String> {
    serde_json::from_str(json).map_err(|e| e.to_string())
}

/// Serializes a value into a pretty-printed JSON string.
pub fn pretty<T: Serialize>(value: &T) -> Result<String, String> {
    serde_json::to_string_pretty(value).map_err(|e| e.to_string())
}
