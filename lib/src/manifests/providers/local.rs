use super::{ManifestProvider, ManifestProviderError};

use std::path::PathBuf;

#[derive(Debug)]
pub struct LocalManifestProvider;

impl ManifestProvider for LocalManifestProvider {
    /// The Local provider is essentially our final chance to
    /// resolve the url. It'll match anything and try to find a
    /// directory
    fn looks_familiar(&self, _url: &str) -> bool {
        true
    }

    fn resolve(&self, url: &str) -> Result<PathBuf, ManifestProviderError> {
        let expanded = shellexpand::tilde(url);
        PathBuf::from(expanded.as_ref())
            .canonicalize()
            .map_err(|_| ManifestProviderError::NoResolution)
    }
}

#[cfg(test)]
mod test {
    use super::super::{ManifestProvider, ManifestProviderError};
    use super::LocalManifestProvider;

    #[test]
    fn test_resolve_absolute_url() {
        let local_manifest_provider = LocalManifestProvider;

        let cwd = std::env::current_dir().unwrap().canonicalize().unwrap();
        let cwd_string = String::from(cwd.to_str().unwrap());

        assert_eq!(cwd, local_manifest_provider.resolve(&cwd_string).unwrap());

        assert_eq!(
            Err(ManifestProviderError::NoResolution),
            local_manifest_provider.resolve(&String::from("/never-resolve"))
        );
    }

    #[test]
    fn test_resolve_relative_url() {
        let local_manifest_provider = LocalManifestProvider {};

        let cwd = std::env::current_dir()
            .unwrap()
            .canonicalize()
            .unwrap()
            .parent()
            .unwrap()
            .join("examples");

        assert_eq!(
            cwd,
            local_manifest_provider
                .resolve(&String::from("../examples"))
                .unwrap()
        );

        assert_eq!(
            Err(ManifestProviderError::NoResolution),
            local_manifest_provider.resolve(&String::from("never-resolve"))
        );
    }

    #[test]
    fn test_resolve_tilde_expands_home() {
        let provider = LocalManifestProvider;
        let home = std::env::var("HOME").unwrap();
        // "~" alone should resolve to the home directory (which exists)
        assert_eq!(
            std::path::PathBuf::from(&home).canonicalize().unwrap(),
            provider.resolve("~").unwrap()
        );
    }

    #[test]
    fn test_resolve_tilde_nonexistent_path() {
        let provider = LocalManifestProvider;
        // Tilde expands but path doesn't exist → NoResolution
        assert_eq!(
            Err(ManifestProviderError::NoResolution),
            provider.resolve("~/etch-test-nonexistent-path-xyz-abc")
        );
    }
}
