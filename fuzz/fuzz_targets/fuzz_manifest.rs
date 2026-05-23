#![no_main]

use etch_lib::manifests::Manifest;
use libfuzzer_sys::fuzz_target;

fuzz_target!(|data: &[u8]| {
    if let Ok(s) = std::str::from_utf8(data) {
        let _ = serde_yaml_ng::from_str::<Manifest>(s);
        let _ = toml::from_str::<Manifest>(s);
    }
});
