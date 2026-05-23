use super::Manifest;
use crate::{
    contexts::{to_tera, Contexts},
    manifests::get_manifest_name,
    tera_functions::register_functions,
};
use ignore::WalkBuilder;
use std::{collections::HashMap, ffi::OsStr, fs::canonicalize, ops::Deref, path::PathBuf};
use tera::Tera;
use tracing::{error, span};

pub fn load(
    manifest_path: PathBuf,
    contexts: &Contexts,
) -> anyhow::Result<HashMap<String, Manifest>> {
    let mut manifests: HashMap<String, Manifest> = HashMap::new();
    let mut parse_error: Option<anyhow::Error> = None;

    let mut walker = WalkBuilder::new(&manifest_path);

    // FIXME: get rid of all .unwrap() calls
    walker
        .standard_filters(true)
        .follow_links(false)
        .same_file_system(true)
        // Arbitrary for now, 9 "should" be enough?
        .max_depth(Some(9))
        .filter_entry(|entry| {
            !(entry.file_type().is_some_and(|ft| ft.is_dir())
            && entry.file_name() == OsStr::new("files"))
        })
        .build()
        // Don't walk directories
        .filter(|entry| {
            !entry
                .as_ref()
                .ok()
                .and_then(|entry| entry.metadata().ok().map(|entry| entry.is_dir()))
                .unwrap_or(false)
        })
        .filter(|entry| {
            entry
                .as_ref()
                .ok()
                .and_then(|entry| entry.file_name().to_str())
                .map(|file_name| {
                    file_name.ends_with(".yaml")
                        || file_name.ends_with(".yml")
                        || file_name.ends_with(".toml")
                })
                .unwrap_or(false)
        })
        .for_each(|entry| {
            if let Ok(filename) = entry {
                let span = span!(
                    tracing::Level::INFO,
                    "manifest_load",
                    manifest = filename.file_name().to_str()
                )
                .entered();

                let entry = canonicalize(filename.into_path()).ok().unwrap_or_default();
                let contents =
                    std::fs::read_to_string(entry.clone()).unwrap_or_else(|_| String::from(""));
                let template = contents.as_str();

                let mut tera = Tera::default();
                register_functions(&mut tera);

                let template = match tera.render_str(template, &to_tera(contexts)) {
                    Ok(template) => template,
                    Err(err) => {
                        let chain = format!("{:#}", anyhow::Error::from(err));
                        error!(message = chain.as_str());
                        span.exit();
                        return;
                    }
                };

                let manifest: anyhow::Result<Manifest> = match entry
                    .extension()
                    .and_then(OsStr::to_str)
                {
                    Some("yaml") | Some("yml") => {
                        serde_yaml_ng::from_str::<Manifest>(template.deref()).map_err(anyhow::Error::from)
                    }
                    Some("toml") => toml::from_str::<Manifest>(template.deref()).map_err(anyhow::Error::from),
                    _ => {
                        error!("Unrecognized file extension for manifest");
                        span.exit();

                        return;
                    }
                };

                match manifest {
                    Ok(mut manifest) => {
                        let name = get_manifest_name(&manifest_path, &entry)
                            .expect("Failed to get manifest name");

                        manifest.root_dir = entry.parent().map(|parent| parent.to_path_buf());

                        manifest.name = Some(name.clone());

                        manifests.insert(name, manifest);
                    }
                    Err(err) => {
                        let manifest_name =
                            get_manifest_name(&manifest_path, &entry).unwrap_or_default();

                        error!("Manifest '{manifest_name}' in file with path '{}' cannot be parsed. Reason: {err}", &entry.display());
                        parse_error = Some(anyhow::anyhow!(
                            "Manifest '{manifest_name}' cannot be parsed: {err}"
                        ));
                    }
                }

                span.exit();
            }
        });

    if let Some(err) = parse_error {
        return Err(err);
    }

    Ok(manifests)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::Config;
    use crate::contexts::build_contexts;
    use tempfile::tempdir;
    use tracing_test::traced_test;

    #[test]
    fn load_yaml_manifest_from_directory() {
        let dir = tempdir().unwrap();
        // Canonicalize the dir to match what load() does internally
        let real_dir = dir.path().canonicalize().unwrap();
        let yaml_path = real_dir.join("main.yaml");
        std::fs::write(&yaml_path, "actions: []\n").unwrap();

        let contexts = build_contexts(&Config::default());
        let manifests = load(real_dir, &contexts).unwrap();
        assert!(!manifests.is_empty());
    }

    #[test]
    fn load_returns_error_for_invalid_yaml() {
        let dir = tempdir().unwrap();
        let real_dir = dir.path().canonicalize().unwrap();
        let bad_yaml = real_dir.join("bad.yaml");
        std::fs::write(&bad_yaml, "this: is: not: valid: yaml: [").unwrap();

        let contexts = build_contexts(&Config::default());
        let result = load(real_dir, &contexts);
        assert!(
            result.is_err(),
            "invalid YAML must return Err, not silently succeed"
        );
    }

    #[test]
    fn load_empty_directory_returns_empty() {
        let dir = tempdir().unwrap();
        let contexts = build_contexts(&Config::default());
        let manifests = load(dir.path().to_path_buf(), &contexts).unwrap();
        assert!(manifests.is_empty());
    }

    #[test]
    fn load_ignores_non_yaml_files() {
        let dir = tempdir().unwrap();
        let real_dir = dir.path().canonicalize().unwrap();
        let txt_path = real_dir.join("notes.txt");
        std::fs::write(&txt_path, "not a manifest").unwrap();

        let contexts = build_contexts(&Config::default());
        let manifests = load(real_dir, &contexts).unwrap();
        assert!(manifests.is_empty());
    }

    #[test]
    fn load_with_yml_extension() {
        let dir = tempdir().unwrap();
        let real_dir = dir.path().canonicalize().unwrap();
        let yml_path = real_dir.join("main.yml");
        std::fs::write(&yml_path, "actions: []\n").unwrap();

        let contexts = build_contexts(&Config::default());
        let manifests = load(real_dir, &contexts).unwrap();
        assert!(!manifests.is_empty());
    }

    #[traced_test]
    #[test]
    fn load_skips_manifest_with_tera_render_error_and_logs_cause() {
        let dir = tempdir().unwrap();
        let real_dir = dir.path().canonicalize().unwrap();
        let bad_tera = real_dir.join("bad.yaml");
        // read_file_contents with a nonexistent path causes a Tera render error
        std::fs::write(
            &bad_tera,
            "{{ read_file_contents(path=\"/no/such/path/abc123xyz\") }}\nactions: []\n",
        )
        .unwrap();

        let contexts = build_contexts(&Config::default());
        // Tera render errors are still silently skipped — load returns Ok(empty)
        let manifests = load(real_dir, &contexts).unwrap();

        assert!(manifests.is_empty());
        // The error message must contain the actual cause, not be silently dropped
        assert!(
            logs_contain("os error") || logs_contain("No such file") || logs_contain("not found"),
            "error log should contain the IO error cause"
        );
    }
}
