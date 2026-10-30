use std::path::Path;

/// `make schema` writes the binary's stdout to schema.json, so the binary must
/// print exactly what `render()` returns and exit 0.
#[test]
fn binary_prints_the_rendered_schema() {
    let output = std::process::Command::new(env!("CARGO_BIN_EXE_jsonschemagen"))
        .output()
        .unwrap();

    assert!(output.status.success(), "exit: {}", output.status);
    assert_eq!(
        String::from_utf8(output.stdout).unwrap(),
        jsonschemagen::render().unwrap()
    );
}

/// Editor autocomplete and etch-config's action-name list read the committed
/// `schema.json`, so it must match what the current source generates.
#[test]
fn committed_schema_matches_the_generated_schema() {
    let path = Path::new(env!("CARGO_MANIFEST_DIR")).join("schema.json");
    let committed = std::fs::read_to_string(&path).unwrap();
    let generated = jsonschemagen::render().unwrap();

    if committed != generated {
        let first_diff = committed
            .lines()
            .zip(generated.lines())
            .enumerate()
            .find(|(_, (c, g))| c != g)
            .map_or_else(
                || String::from("one file is a prefix of the other"),
                |(i, (c, g))| {
                    format!(
                        "first difference at line {}: committed {c:?}, generated {g:?}",
                        i + 1
                    )
                },
            );
        panic!(
            "{} is stale ({first_diff}); regenerate it with `make schema`",
            path.display()
        );
    }
}

#[test]
fn generated_schema_does_not_depend_on_the_directory_it_runs_in() {
    // A default computed at run time (command.run's `dir` is the cwd) must not
    // be baked into the schema, or every machine and checkout generates a
    // different file.
    let cwd = std::env::current_dir().unwrap();
    let generated = jsonschemagen::render().unwrap();

    assert!(
        !generated.contains(&*cwd.to_string_lossy()),
        "schema embeds the cwd {}",
        cwd.display()
    );
}

fn provider_defaults(value: &serde_json::Value, found: &mut Vec<String>) {
    match value {
        serde_json::Value::Object(map) => {
            let is_provider = map
                .get("$ref")
                .and_then(|r| r.as_str())
                .is_some_and(|r| r.ends_with("Providers"));
            if is_provider {
                if let Some(default) = map.get("default") {
                    found.push(format!("{} = {default}", map["$ref"]));
                }
            }
            map.values().for_each(|v| provider_defaults(v, found));
        }
        serde_json::Value::Array(items) => items.iter().for_each(|v| provider_defaults(v, found)),
        _ => {}
    }
}

#[test]
fn generated_schema_does_not_depend_on_the_host_os() {
    // Provider defaults are chosen from the host (apt on Ubuntu, brew on
    // macOS), so a schema carrying them differs by the machine generating it.
    let generated: serde_json::Value =
        serde_json::from_str(&jsonschemagen::render().unwrap()).unwrap();
    let mut found = Vec::new();
    provider_defaults(&generated, &mut found);

    assert!(
        found.is_empty(),
        "host-derived provider defaults: {found:?}"
    );
}

#[test]
fn generated_schema_names_actions_by_their_yaml_name() {
    let generated = jsonschemagen::render().unwrap();

    assert!(generated.contains("\"command.run\""), "no YAML action name");
    assert!(
        !generated.contains("\"CommandRun\""),
        "Rust variant name leaked"
    );
}
