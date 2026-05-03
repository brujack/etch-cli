use crate::contexts::Contexts;
use crate::manifests::Manifest;
use std::path::Path;

pub fn make_manifest(dir: &Path) -> Manifest {
    Manifest {
        root_dir: Some(dir.to_path_buf()),
        ..Default::default()
    }
}

pub fn make_contexts() -> Contexts {
    Contexts::default()
}
