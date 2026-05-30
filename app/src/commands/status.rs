use crate::Runtime;
use colored::Colorize;
use etch_lib::atoms::AtomStatus;
use etch_lib::manifests::load;
use rhai::Engine;
use serde::Serialize;
use tracing::warn;

use super::apply::Apply;

/// Per-atom result collected during a status run.
#[derive(Debug, Serialize)]
pub struct AtomResult {
    pub label: String,
    #[serde(flatten)]
    pub status: AtomStatus,
}

/// Per-manifest result collected during a status run.
#[derive(Debug, Serialize)]
pub struct ManifestResult {
    pub name: String,
    pub atoms: Vec<AtomResult>,
}

#[derive(Debug, Serialize)]
struct Summary {
    ok: usize,
    unchecked: usize,
    drifted: usize,
    missing: usize,
}

#[derive(Debug, Serialize)]
struct JsonOutput {
    manifests: Vec<ManifestResult>,
    summary: Summary,
}

pub fn run_status(apply: &Apply, runtime: &Runtime) -> anyhow::Result<()> {
    use etch_lib::contexts::to_rhai;

    let contexts = &runtime.contexts;
    let manifest_path = apply.manifest_path(runtime)?;
    let manifests = load(manifest_path, contexts)?;

    let engine = Engine::new();
    let mut scope = to_rhai(contexts);

    let mut results: Vec<ManifestResult> = Vec::new();

    for (name, manifest) in manifests.iter() {
        // Apply manifest-level where condition
        if let Some(where_condition) = &manifest.r#where {
            match engine.eval_with_scope::<bool>(&mut scope, where_condition) {
                Ok(false) | Err(_) => continue,
                Ok(true) => {}
            }
        }

        let mut atom_results: Vec<AtomResult> = Vec::new();

        for raw_action in manifest.actions.iter() {
            let action = raw_action.inner_ref();
            let steps = match action.plan(manifest, contexts) {
                Ok(s) => s,
                Err(err) => {
                    warn!("status: action plan failed in {}: {:?}", name, err);
                    continue;
                }
            };

            for step in steps {
                let label = format!("{}", step.atom);
                let atom_status = match step.atom.status() {
                    Ok(s) => s,
                    Err(err) => {
                        warn!("status: atom status check failed: {:?}", err);
                        AtomStatus::Unchecked
                    }
                };
                atom_results.push(AtomResult {
                    label,
                    status: atom_status,
                });
            }
        }

        results.push(ManifestResult {
            name: name.clone(),
            atoms: atom_results,
        });
    }

    let mut total_ok = 0usize;
    let mut total_unchecked = 0usize;
    let mut total_drifted = 0usize;
    let mut total_missing = 0usize;

    for r in &results {
        for a in &r.atoms {
            match &a.status {
                AtomStatus::Ok => total_ok += 1,
                AtomStatus::Unchecked => total_unchecked += 1,
                AtomStatus::Drifted { .. } => total_drifted += 1,
                AtomStatus::Missing => total_missing += 1,
            }
        }
    }

    if apply.json {
        let output = JsonOutput {
            manifests: results,
            summary: Summary {
                ok: total_ok,
                unchecked: total_unchecked,
                drifted: total_drifted,
                missing: total_missing,
            },
        };
        println!("{}", serde_json::to_string_pretty(&output)?);
    } else {
        for r in &results {
            println!("manifest: {}", r.name.bold());
            for a in &r.atoms {
                let (icon, detail) = match &a.status {
                    AtomStatus::Ok => ("✓ ok".green().to_string(), String::new()),
                    AtomStatus::Unchecked => {
                        ("✓ ok (unchecked)".dimmed().to_string(), String::new())
                    }
                    AtomStatus::Drifted { expected, actual } => (
                        "✗ drifted".red().to_string(),
                        format!(
                            "\n    {} expected → {}\n    {} actual   → {}",
                            " ".dimmed(),
                            expected.yellow(),
                            " ".dimmed(),
                            actual.red(),
                        ),
                    ),
                    AtomStatus::Missing => ("✗ missing".red().to_string(), String::new()),
                };
                println!("  {:<60}  {}{}", a.label.dimmed(), icon, detail);
            }
        }
        println!();

        let ok_str = if total_unchecked > 0 {
            format!(
                "{} ok ({} unchecked)",
                total_ok + total_unchecked,
                total_unchecked
            )
        } else {
            format!("{} ok", total_ok)
        };
        println!(
            "Summary: {}, {} drifted, {} missing",
            ok_str,
            if total_drifted > 0 {
                total_drifted.to_string().red().to_string()
            } else {
                total_drifted.to_string()
            },
            if total_missing > 0 {
                total_missing.to_string().red().to_string()
            } else {
                total_missing.to_string()
            },
        );
    }

    if total_drifted > 0 || total_missing > 0 {
        return Err(anyhow::anyhow!("drift detected"));
    }

    Ok(())
}
