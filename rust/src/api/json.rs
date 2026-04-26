use serde::Serialize;
use serde::de::DeserializeOwned;
use std::fs;
use std::path::Path;

/// Reads and parses a JSON file into the specified type.
pub fn read_json_file<T: DeserializeOwned>(path: impl AsRef<Path>) -> Result<T, String> {
    let content = fs::read_to_string(path).map_err(|e| e.to_string())?;
    serde_json::from_str(&content).map_err(|e| e.to_string())
}

/// Serializes and writes a value to a JSON file.
pub fn write_json_file<T: Serialize>(path: impl AsRef<Path>, value: &T) -> Result<(), String> {
    let content = serde_json::to_string_pretty(value).map_err(|e| e.to_string())?;
    fs::write(path, content).map_err(|e| e.to_string())
}

/// Serializes and writes a value to a JSON file atomically (write to .tmp then rename).
pub fn write_json_file_atomic<T: Serialize>(
    path: impl AsRef<Path>,
    value: &T,
) -> Result<(), String> {
    let path = path.as_ref();
    let temp_path = path.with_extension("tmp");
    write_json_file(&temp_path, value)?;
    fs::rename(temp_path, path).map_err(|e| e.to_string())
}

/// Parses a JSON string into the specified type.
pub fn parse_json<T: DeserializeOwned>(json: &str) -> Result<T, String> {
    serde_json::from_str(json).map_err(|e| e.to_string())
}

/// Serializes a value into a pretty-printed JSON string.
pub fn to_pretty_json<T: Serialize>(value: &T) -> Result<String, String> {
    serde_json::to_string_pretty(value).map_err(|e| e.to_string())
}

/// Validates that a JSON string can be parsed into the specified type.
pub fn validate_json<T: DeserializeOwned>(json: &str) -> bool {
    serde_json::from_str::<T>(json).is_ok()
}
