use std::collections::HashMap;

use anyhow::Result;
use reqwest::Url;
use toml::Value;

pub fn toml_values(url: &Url, contexts: &mut HashMap<String, String>) -> Result<()> {
    let path = url.path();

    let contents = std::fs::read_to_string(path)?;
    let values: HashMap<String, Value> = toml::from_str(&contents)?;

    for (key, value) in values {
        contexts.insert(key.to_string(), value.to_string());
    }

    Ok(())
}

pub fn yaml_values(url: &Url, contexts: &mut HashMap<String, String>) -> Result<()> {
    let path = url.path();

    let contents = std::fs::read_to_string(path)?;
    let values: HashMap<String, Value> = serde_yaml_ng::from_str(&contents)?;

    for (key, value) in values {
        contexts.insert(key.to_string(), value.to_string());
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use tempfile::NamedTempFile;

    fn file_url(path: &str) -> Url {
        Url::parse(&format!("file+toml://{path}")).unwrap()
    }

    fn yaml_file_url(path: &str) -> Url {
        Url::parse(&format!("file+yaml://{path}")).unwrap()
    }

    #[test]
    fn toml_values_reads_key_value_pairs() -> Result<()> {
        let mut file = NamedTempFile::new()?;
        writeln!(file.as_file_mut(), "ship = \"Daedalus\"\ncrew = \"42\"")?;

        let path = file.path().to_str().unwrap().to_string();
        let url = file_url(&path);

        let mut contexts = HashMap::new();
        toml_values(&url, &mut contexts)?;

        assert_eq!(
            contexts.get("ship").map(String::as_str),
            Some("\"Daedalus\"")
        );
        Ok(())
    }

    #[test]
    fn toml_values_nonexistent_file_errors() {
        let url = file_url("/nonexistent/path/file.toml");
        let mut contexts = HashMap::new();
        assert!(toml_values(&url, &mut contexts).is_err());
    }

    #[test]
    fn yaml_values_reads_key_value_pairs() -> Result<()> {
        let mut file = NamedTempFile::new()?;
        writeln!(file.as_file_mut(), "ship: Prometheus\ncrew: 100")?;

        let path = file.path().to_str().unwrap().to_string();
        let url = yaml_file_url(&path);

        let mut contexts = HashMap::new();
        yaml_values(&url, &mut contexts)?;

        assert!(contexts.contains_key("ship"));
        Ok(())
    }

    #[test]
    fn yaml_values_nonexistent_file_errors() {
        let url = yaml_file_url("/nonexistent/path/file.yaml");
        let mut contexts = HashMap::new();
        assert!(yaml_values(&url, &mut contexts).is_err());
    }
}
