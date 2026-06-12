use super::EtchCommand;
use crate::Runtime;
use clap::Parser;
use etch_lib::state::{StateEntry, StateStore};

#[derive(Parser, Debug)]
pub(crate) struct History {
    /// Filter entries by manifest path (substring match)
    #[arg(long)]
    pub manifest: Option<String>,

    /// Output as NDJSON (one JSON object per line)
    #[arg(long)]
    pub json: bool,
}

impl EtchCommand for History {
    #[cfg(not(tarpaulin_include))]
    fn execute(&self, _runtime: &Runtime) -> anyhow::Result<()> {
        let store = StateStore::new();
        let state = match store.load() {
            Ok(s) => s,
            Err(e) => {
                eprintln!("error: cannot read state file: {e:#}");
                std::process::exit(1);
            }
        };

        let entries: Vec<&StateEntry> = state
            .atoms
            .iter()
            .filter(|e| {
                self.manifest
                    .as_deref()
                    .is_none_or(|filter| e.manifest.contains(filter))
            })
            .collect();

        if self.json {
            for entry in entries {
                println!("{}", serde_json::to_string(entry)?);
            }
        } else {
            print_table(&entries);
        }

        Ok(())
    }

    #[cfg(tarpaulin_include)]
    fn execute(&self, _runtime: &Runtime) -> anyhow::Result<()> {
        unreachable!()
    }
}

#[cfg(not(tarpaulin_include))]
pub(crate) fn print_table(entries: &[&StateEntry]) {
    println!(
        "{:<50} {:<15} {:<30} {:<22} CHANGED",
        "MANIFEST", "ACTION", "KEY", "APPLIED AT"
    );
    for entry in entries {
        println!(
            "{:<50} {:<15} {:<30} {:<22} {}",
            truncate(&entry.manifest, 50),
            truncate(&entry.action, 15),
            truncate(&entry.key, 30),
            entry.applied_at.format("%Y-%m-%d %H:%M:%S"),
            if entry.changed { "yes" } else { "no" },
        );
    }
}

fn truncate(s: &str, max: usize) -> &str {
    match s.char_indices().nth(max) {
        Some((i, _)) => &s[..i],
        None => s,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Utc;
    use etch_lib::state::{State, StateEntry};

    fn make_entry(manifest: &str, action: &str, key: &str, changed: bool) -> StateEntry {
        StateEntry {
            manifest: manifest.to_string(),
            action: action.to_string(),
            key: key.to_string(),
            applied_at: Utc::now(),
            sha256: None,
            changed,
        }
    }

    fn render_table(entries: &[StateEntry], manifest_filter: Option<&str>) -> String {
        let refs: Vec<&StateEntry> = entries
            .iter()
            .filter(|e| manifest_filter.is_none_or(|f| e.manifest.contains(f)))
            .collect();
        let mut buf = Vec::new();
        use std::io::Write;
        writeln!(
            buf,
            "{:<50} {:<15} {:<30} {:<22} CHANGED",
            "MANIFEST", "ACTION", "KEY", "APPLIED AT"
        )
        .unwrap();
        for e in &refs {
            writeln!(
                buf,
                "{:<50} {:<15} {:<30} {:<22} {}",
                truncate(&e.manifest, 50),
                truncate(&e.action, 15),
                truncate(&e.key, 30),
                e.applied_at.format("%Y-%m-%d %H:%M:%S"),
                if e.changed { "yes" } else { "no" },
            )
            .unwrap();
        }
        String::from_utf8(buf).unwrap()
    }

    fn render_json(entries: &[StateEntry]) -> String {
        entries
            .iter()
            .map(|e| serde_json::to_string(e).unwrap())
            .collect::<Vec<_>>()
            .join("\n")
    }

    #[test]
    fn table_output_contains_key_and_changed_yes_no() {
        let entries = vec![
            make_entry("core.yaml", "file.copy", "~/.zshrc", true),
            make_entry("tools.yaml", "git.clone", "~/.oh-my-zsh", false),
        ];
        let output = render_table(&entries, None);
        assert!(output.contains("~/.zshrc"), "should contain key");
        assert!(
            output.contains("yes"),
            "should contain yes for changed=true"
        );
        assert!(output.contains("no"), "should contain no for changed=false");
    }

    #[test]
    fn manifest_filter_shows_only_matching_rows() {
        let entries = vec![
            make_entry("core.yaml", "file.copy", "~/.zshrc", true),
            make_entry("tools.yaml", "git.clone", "~/.oh-my-zsh", false),
        ];
        let output = render_table(&entries, Some("core"));
        assert!(output.contains("~/.zshrc"), "core entry should appear");
        assert!(
            !output.contains("~/.oh-my-zsh"),
            "tools entry should be filtered out"
        );
    }

    #[test]
    fn json_output_each_line_valid_json_with_action_field() {
        let entries = vec![make_entry("core.yaml", "file.copy", "~/.zshrc", true)];
        let output = render_json(&entries);
        let line = output.lines().next().unwrap();
        let parsed: serde_json::Value = serde_json::from_str(line).unwrap();
        assert_eq!(parsed["action"], "file.copy");
    }

    #[test]
    fn state_default_has_empty_atoms() {
        let state = State::default();
        assert!(state.atoms.is_empty());
        assert_eq!(state.schema_version, 1);
    }
}
