use super::EtchCommand;
use crate::Runtime;
use clap::Parser;
use colored::Colorize;
use etch_lib::config::Config;
use etch_lib::contexts::Contexts;
use etch_lib::doctor::cred_perms::CredPermsCheck;
use etch_lib::doctor::symlinks::SymlinkCheck;
use etch_lib::doctor::tools::ToolsCheck;
use etch_lib::doctor::versions::VersionsCheck;
use etch_lib::doctor::{CheckResult, DoctorCheck};
use etch_lib::manifests::load;
use serde::Serialize;
use std::collections::HashMap;

#[derive(Parser, Debug, Default)]
pub(crate) struct Doctor {
    /// Output results as JSON
    #[arg(long)]
    pub json: bool,

    /// Only show failing checks
    #[arg(long)]
    pub missing_only: bool,
}

pub(crate) fn run_doctor_checks(
    config: &Config,
    contexts: &Contexts,
) -> anyhow::Result<Vec<(&'static str, Vec<CheckResult>)>> {
    let manifests = if let Some(first) = config.manifest_paths.first() {
        match crate::manifests::resolve(first) {
            Some(path) => load(path, contexts).unwrap_or_default(),
            None => HashMap::new(),
        }
    } else {
        HashMap::new()
    };

    Ok(vec![
        ("Symlinks", SymlinkCheck.run(config, &manifests)),
        ("Tools", ToolsCheck.run(config, &manifests)),
        ("Credential dirs", CredPermsCheck.run(config, &manifests)),
        ("Versions", VersionsCheck.run(config, &manifests)),
    ])
}

#[derive(Serialize)]
struct JsonCheckResult {
    label: String,
    passed: bool,
    detail: Option<String>,
}

#[derive(Serialize)]
struct JsonOutput {
    checks: Vec<JsonCheckResult>,
    summary: JsonSummary,
}

#[derive(Serialize)]
struct JsonSummary {
    passed: usize,
    failed: usize,
}

fn render_human(sections: &[(&'static str, Vec<CheckResult>)], missing_only: bool) {
    let total_passed = sections
        .iter()
        .flat_map(|(_, r)| r.iter())
        .filter(|r| r.passed)
        .count();
    let total_failed = sections
        .iter()
        .flat_map(|(_, r)| r.iter())
        .filter(|r| !r.passed)
        .count();

    for (section_name, results) in sections {
        if results.is_empty() {
            continue;
        }
        let has_failures = results.iter().any(|r| !r.passed);
        if missing_only && !has_failures {
            continue;
        }
        println!("{section_name}");
        for r in results {
            if missing_only && r.passed {
                continue;
            }
            if r.passed {
                println!("  {} {}", "✓".green(), r.label);
            } else {
                let detail = r.detail.as_deref().unwrap_or("failed");
                println!("  {} {}  [{}]", "✗".red(), r.label, detail);
            }
        }
        println!();
    }
    println!("{total_passed} passed, {total_failed} failed");
}

fn render_json(sections: &[(&'static str, Vec<CheckResult>)]) -> anyhow::Result<()> {
    let checks: Vec<JsonCheckResult> = sections
        .iter()
        .flat_map(|(_, results)| {
            results.iter().map(|r| JsonCheckResult {
                label: r.label.clone(),
                passed: r.passed,
                detail: r.detail.clone(),
            })
        })
        .collect();
    let passed = checks.iter().filter(|c| c.passed).count();
    let failed = checks.len() - passed;
    let output = JsonOutput {
        checks,
        summary: JsonSummary { passed, failed },
    };
    println!("{}", serde_json::to_string_pretty(&output)?);
    Ok(())
}

impl EtchCommand for Doctor {
    #[cfg(not(tarpaulin_include))]
    fn execute(&self, runtime: &Runtime) -> anyhow::Result<()> {
        let sections = run_doctor_checks(&runtime.config, &runtime.contexts)?;
        let any_failed = sections
            .iter()
            .flat_map(|(_, r)| r.iter())
            .any(|r| !r.passed);

        if self.json {
            render_json(&sections)?;
        } else {
            render_human(&sections, self.missing_only);
        }

        if any_failed {
            std::process::exit(1);
        }
        Ok(())
    }

    #[cfg(tarpaulin_include)]
    fn execute(&self, _runtime: &Runtime) -> anyhow::Result<()> {
        unreachable!()
    }
}
