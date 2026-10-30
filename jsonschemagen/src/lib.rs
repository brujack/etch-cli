use etch_lib::manifests::Manifest;
use schemars::schema_for;

/// Renders the manifest JSON Schema exactly as `schema.json` holds it: pretty
/// printed, with a trailing newline.
pub fn render() -> serde_json::Result<String> {
    let schema = schema_for!(Manifest);
    Ok(format!("{}\n", serde_json::to_string_pretty(&schema)?))
}
