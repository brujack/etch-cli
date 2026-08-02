use std::collections::HashMap;

use anyhow::Result;
use reqwest::Url;

use crate::{config::Config, contexts::Context, contexts::ContextProvider};

pub mod dns;
pub mod file;

#[derive(Debug)]
pub struct VariableIncludeContextProvider<'a> {
    pub config: &'a Config,
}

impl<'a> ContextProvider for VariableIncludeContextProvider<'a> {
    fn get_prefix(&self) -> String {
        String::from("include_variables")
    }

    fn get_contexts(&self) -> Result<Vec<super::Context>> {
        let mut contexts = HashMap::<String, String>::new();

        if let Some(variable_includes) = &self.config.include_variables {
            for variable_include in variable_includes {
                let url = Url::parse(variable_include)?;

                if url.scheme() == "dns+txt" {
                    dns::txt_record_values(&url, &mut contexts)?;
                } else if url.scheme() == "file+toml" {
                    file::toml_values(&url, &mut contexts)?;
                } else if url.scheme() == "file+yaml" {
                    file::yaml_values(&url, &mut contexts)?;
                } else {
                    return Err(anyhow::anyhow!(
                        "Unknown variable include scheme: {}",
                        url.scheme()
                    ));
                }
            }
        }

        let contexts = contexts
            .into_iter()
            .map(|(key, value)| Context::KeyValueContext(key, value.into()))
            .collect::<Vec<_>>();

        Ok(contexts)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::Config;
    use std::io::Write;
    use tempfile::NamedTempFile;

    #[test]
    fn no_variable_includes_returns_empty() -> Result<()> {
        let config = Config {
            include_variables: None,
            ..Default::default()
        };
        let provider = VariableIncludeContextProvider { config: &config };
        let contexts = provider.get_contexts()?;
        assert!(contexts.is_empty());
        Ok(())
    }

    #[test]
    fn file_toml_include_loads_values() -> Result<()> {
        let mut file = NamedTempFile::new()?;
        writeln!(file.as_file_mut(), "ship = \"Prometheus\"")?;
        let path = file.path().to_str().unwrap().to_string();

        let config = Config {
            include_variables: Some(vec![format!("file+toml://{path}")]),
            ..Default::default()
        };
        let provider = VariableIncludeContextProvider { config: &config };
        let contexts = provider.get_contexts()?;
        assert!(!contexts.is_empty());
        let found = contexts
            .iter()
            .any(|c| matches!(c, Context::KeyValueContext(k, _) if k == "ship"));
        assert!(found);
        Ok(())
    }

    #[test]
    fn file_yaml_include_loads_values() -> Result<()> {
        let mut file = NamedTempFile::new()?;
        writeln!(file.as_file_mut(), "planet: Lantea")?;
        let path = file.path().to_str().unwrap().to_string();

        let config = Config {
            include_variables: Some(vec![format!("file+yaml://{path}")]),
            ..Default::default()
        };
        let provider = VariableIncludeContextProvider { config: &config };
        let contexts = provider.get_contexts()?;
        assert!(!contexts.is_empty());
        Ok(())
    }

    #[test]
    fn unknown_scheme_returns_error() {
        let config = Config {
            include_variables: Some(vec!["http://example.com/vars".to_string()]),
            ..Default::default()
        };
        let provider = VariableIncludeContextProvider { config: &config };
        assert!(provider.get_contexts().is_err());
    }

    #[test]
    fn get_prefix_is_include_variables() {
        let config = Config::default();
        let provider = VariableIncludeContextProvider { config: &config };
        assert_eq!(provider.get_prefix(), "include_variables");
    }
}
