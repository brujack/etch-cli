#![no_main]

use etch_lib::actions::{FileAction, FileLink};
use etch_lib::manifests::Manifest;
use libfuzzer_sys::fuzz_target;
use std::path::PathBuf;

fn stub_manifest() -> Manifest {
    Manifest {
        root_dir: Some(PathBuf::from(std::env::temp_dir())),
        ..Default::default()
    }
}

fuzz_target!(|data: &[u8]| {
    if let Ok(s) = std::str::from_utf8(data) {
        let manifest = stub_manifest();
        let action = FileLink::default();
        let _ = action.resolve(&manifest, s);
    }
});
