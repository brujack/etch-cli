use std::collections::HashMap;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

use clap::Parser;
use tracing::{info, warn};

use etch_lib::config::ClaudeUpdateConfig;

use super::EtchCommand;
use crate::Runtime;

// ── Section order (fixed for summary display) ─────────────────────────────────
const SECTION_ORDER: &[&str] = &[
    "brew",
    "softwareupdate",
    "mas",
    "claude",
    "npm",
    "apt",
    "snap",
    "pip",
    "rust",
    "ai-config",
    "dotfiles",
    "oh-my-zsh",
    "tpm",
    "tfenv",
    "gems",
    "cheat.sh",
];

// ── Types ─────────────────────────────────────────────────────────────────────

pub(crate) enum StepStatus {
    Ok(String),
    Fail(String),
    Skip(String),
    #[allow(dead_code)]
    Warn(String),
}

impl StepStatus {
    fn tag(&self) -> &'static str {
        match self {
            Self::Ok(_) => "[OK]  ",
            Self::Fail(_) => "[FAIL]",
            Self::Skip(_) => "[SKIP]",
            Self::Warn(_) => "[WARN]",
        }
    }

    fn detail(&self) -> &str {
        match self {
            Self::Ok(s) | Self::Fail(s) | Self::Skip(s) | Self::Warn(s) => s,
        }
    }
}

pub(crate) struct UpdateStepResult {
    pub name: &'static str,
    pub status: StepStatus,
}

// ── CLI struct ────────────────────────────────────────────────────────────────

#[derive(Parser, Debug, Default)]
pub(crate) struct Update {
    /// Run only these categories (comma-separated: brew,rust)
    #[arg(long, value_delimiter = ',', conflicts_with = "skip")]
    pub only: Vec<String>,

    /// Skip these categories (comma-separated: pip,gems)
    #[arg(long, value_delimiter = ',', conflicts_with = "only")]
    pub skip: Vec<String>,
}

const VALID_CATEGORIES: &[&str] = &[
    "brew",
    "system",
    "mas",
    "claude",
    "packages",
    "pip",
    "rust",
    "git-tools",
    "gems",
    "cheatsh",
];

impl Update {
    fn should_run(&self, category: &str) -> bool {
        if !self.only.is_empty() {
            self.only.iter().any(|c| c == category)
        } else if !self.skip.is_empty() {
            !self.skip.iter().any(|c| c == category)
        } else {
            true
        }
    }

    fn validate_categories(&self) -> anyhow::Result<()> {
        let all = self.only.iter().chain(self.skip.iter());
        for name in all {
            if !VALID_CATEGORIES.contains(&name.as_str()) {
                anyhow::bail!(
                    "unknown category '{name}'; valid: {}",
                    VALID_CATEGORIES.join(", ")
                );
            }
        }
        Ok(())
    }
}

// ── Helpers ───────────────────────────────────────────────────────────────────

fn format_timestamp() -> String {
    Command::new("date")
        .arg("+%Y-%m-%d %H:%M:%S")
        .output()
        .ok()
        .filter(|o| o.status.success())
        .map(|o| String::from_utf8_lossy(&o.stdout).trim().to_string())
        .unwrap_or_else(|| "unknown time".to_string())
}

#[cfg(not(tarpaulin_include))]
fn run_cmd(cmd: &str, args: &[&str]) -> i32 {
    Command::new(cmd)
        .args(args)
        .status()
        .map(|s| s.code().unwrap_or(1))
        .unwrap_or(1)
}

#[cfg(not(tarpaulin_include))]
fn capture(cmd: &str, args: &[&str]) -> Vec<String> {
    Command::new(cmd)
        .args(args)
        .output()
        .ok()
        .filter(|o| o.status.success())
        .map(|o| {
            String::from_utf8_lossy(&o.stdout)
                .lines()
                .filter(|l| !l.is_empty())
                .map(String::from)
                .collect()
        })
        .unwrap_or_default()
}

fn diff_lines(pre: &[String], post: &[String]) -> Vec<String> {
    let pre_set: std::collections::HashSet<&str> = pre.iter().map(String::as_str).collect();
    post.iter()
        .filter(|l| !pre_set.contains(l.as_str()))
        .cloned()
        .collect()
}

#[cfg(not(tarpaulin_include))]
fn git_commit_count(dir: &Path, old_sha: &str) -> usize {
    Command::new("git")
        .args([
            "-C",
            &dir.to_string_lossy(),
            "log",
            &format!("{old_sha}..HEAD"),
            "--oneline",
        ])
        .output()
        .ok()
        .map(|o| {
            String::from_utf8_lossy(&o.stdout)
                .lines()
                .filter(|l| !l.is_empty())
                .count()
        })
        .unwrap_or(0)
}

fn home_dir() -> PathBuf {
    dirs_next::home_dir().unwrap_or_else(|| PathBuf::from("/tmp"))
}

// Pure helper — testable without I/O. Takes pre-resolved booleans from has_cmd().
fn select_python_cmd(
    pyenv_shim: &Path,
    has_python3: bool,
    has_python: bool,
) -> Option<(String, Vec<&'static str>)> {
    if pyenv_shim.exists() {
        Some((pyenv_shim.to_string_lossy().into_owned(), vec![]))
    } else if has_python3 {
        Some((
            "python3".to_string(),
            vec!["--user", "--break-system-packages"],
        ))
    } else if has_python {
        Some((
            "python".to_string(),
            vec!["--user", "--break-system-packages"],
        ))
    } else {
        None
    }
}

// Pure helper — testable without I/O.
fn select_gem_cmd(rbenv_shim: &Path, has_gem: bool) -> Option<(String, Vec<&'static str>)> {
    if rbenv_shim.exists() {
        Some((rbenv_shim.to_string_lossy().into_owned(), vec![]))
    } else if has_gem {
        Some(("gem".to_string(), vec!["--user-install"]))
    } else {
        None
    }
}

// When using system Python (non-empty pip_extra), also scope the outdated list to
// user packages — otherwise pip reports system-managed packages as outdated and we
// attempt to user-install them, producing version conflicts.
fn pip_outdated_args(pip_extra: &[&str]) -> Vec<&'static str> {
    let mut args = vec!["-m", "pip", "list", "--outdated", "--format=columns"];
    if !pip_extra.is_empty() {
        args.push("--user");
    }
    args
}

fn expand_tilde(p: &str) -> PathBuf {
    if p == "~" {
        home_dir()
    } else if let Some(rest) = p.strip_prefix("~/") {
        home_dir().join(rest)
    } else {
        PathBuf::from(p)
    }
}

#[cfg(not(tarpaulin_include))]
fn has_cmd(cmd: &str) -> bool {
    Command::new("which")
        .arg(cmd)
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .map(|s| s.success())
        .unwrap_or(false)
}

#[cfg(not(tarpaulin_include))]
fn skip_result(name: &'static str, reason: &str) -> UpdateStepResult {
    UpdateStepResult {
        name,
        status: StepStatus::Skip(reason.to_string()),
    }
}

#[cfg(not(tarpaulin_include))]
fn fail_result(name: &'static str, exit: i32) -> UpdateStepResult {
    UpdateStepResult {
        name,
        status: StepStatus::Fail(format!("exit {exit}")),
    }
}

// ── Step functions ────────────────────────────────────────────────────────────

#[cfg(not(tarpaulin_include))]
fn update_brew() -> UpdateStepResult {
    if !has_cmd("brew") {
        return skip_result("brew", "brew not installed");
    }

    let pre_formula = capture("brew", &["list", "--formula", "--versions"]);
    let pre_cask = if std::env::consts::OS == "macos" {
        capture("brew", &["list", "--cask", "--versions"])
    } else {
        vec![]
    };

    let exit = run_cmd("brew", &["upgrade"]);
    if exit != 0 {
        return fail_result("brew", exit);
    }
    let _ = run_cmd("brew", &["cleanup"]);

    let post_formula = capture("brew", &["list", "--formula", "--versions"]);
    let post_cask = if std::env::consts::OS == "macos" {
        capture("brew", &["list", "--cask", "--versions"])
    } else {
        vec![]
    };

    let formula_diff = diff_lines(&pre_formula, &post_formula);
    let cask_diff = diff_lines(&pre_cask, &post_cask);

    let detail = match (formula_diff.len(), cask_diff.len()) {
        (0, 0) => "no changes".to_string(),
        (f, 0) => format!("{f} formulae ({})", formula_diff.join(", ")),
        (0, c) => format!("{c} cask(s) ({})", cask_diff.join(", ")),
        (f, c) => format!(
            "{f} formulae ({}) + {c} cask(s) ({})",
            formula_diff.join(", "),
            cask_diff.join(", ")
        ),
    };

    UpdateStepResult {
        name: "brew",
        status: StepStatus::Ok(detail),
    }
}

#[cfg(not(tarpaulin_include))]
fn update_softwareupdate() -> UpdateStepResult {
    if std::env::consts::OS != "macos" {
        return skip_result("softwareupdate", "not applicable");
    }

    let pending: Vec<String> = Command::new("softwareupdate")
        .arg("-l")
        .output()
        .ok()
        .map(|o| {
            String::from_utf8_lossy(&o.stdout)
                .lines()
                .filter(|l| l.contains("* Label:"))
                .map(|l| l.split("* Label:").nth(1).unwrap_or("").trim().to_string())
                .collect()
        })
        .unwrap_or_default();

    if pending.is_empty() {
        return UpdateStepResult {
            name: "softwareupdate",
            status: StepStatus::Ok("no changes".to_string()),
        };
    }

    let exit = run_cmd("softwareupdate", &["-ia"]);
    let detail = format!("{} update(s) ({})", pending.len(), pending.join(", "));
    UpdateStepResult {
        name: "softwareupdate",
        status: if exit == 0 {
            StepStatus::Ok(detail)
        } else {
            StepStatus::Fail(format!("exit {exit}"))
        },
    }
}

#[cfg(not(tarpaulin_include))]
fn update_mas() -> UpdateStepResult {
    if std::env::consts::OS != "macos" {
        return skip_result("mas", "not applicable");
    }
    if !has_cmd("mas") {
        return skip_result("mas", "mas not installed");
    }

    match Command::new("mas").arg("upgrade").output() {
        Ok(o) if o.status.success() => {
            let text = String::from_utf8_lossy(&o.stdout);
            let updated: Vec<&str> = text
                .lines()
                .filter(|l| l.contains("==> Updated"))
                .map(|l| l.split("==> Updated").nth(1).unwrap_or("").trim())
                .collect();
            let detail = if updated.is_empty() {
                "no changes".to_string()
            } else {
                format!("{} app(s) ({})", updated.len(), updated.join(", "))
            };
            UpdateStepResult {
                name: "mas",
                status: StepStatus::Ok(detail),
            }
        }
        Ok(o) => fail_result("mas", o.status.code().unwrap_or(1)),
        Err(e) => UpdateStepResult {
            name: "mas",
            status: StepStatus::Fail(e.to_string()),
        },
    }
}

/// Extract full `name@marketplace` tokens from `claude plugins list` stdout.
/// Lines matching `❯ <token>` are kept; everything else is ignored.
fn parse_claude_plugin_tokens(output: &str) -> Vec<String> {
    output
        .lines()
        .filter_map(|line| {
            let trimmed = line.trim();
            trimmed
                .strip_prefix('❯')
                .map(|rest| rest.trim().to_string())
                .filter(|s| !s.is_empty())
        })
        .collect()
}

#[cfg(not(tarpaulin_include))]
fn update_claude(_config: &ClaudeUpdateConfig) -> UpdateStepResult {
    if !has_cmd("claude") {
        return skip_result("claude", "claude not installed");
    }

    let list_lines = capture("claude", &["plugins", "list"]);
    let plugins = parse_claude_plugin_tokens(&list_lines.join("\n"));

    if plugins.is_empty() {
        return UpdateStepResult {
            name: "claude",
            status: StepStatus::Ok("no plugins installed".to_string()),
        };
    }

    let pre_versions: Vec<String> = list_lines
        .iter()
        .filter(|l| l.contains("Version:"))
        .cloned()
        .collect();

    let mut fail_count = 0usize;
    for plugin in &plugins {
        let exit = run_cmd("claude", &["plugins", "update", plugin.as_str()]);
        if exit != 0 {
            warn!("claude plugin update failed for {plugin}");
            fail_count += 1;
        }
    }

    let post_versions: Vec<String> = capture("claude", &["plugins", "list"])
        .into_iter()
        .filter(|l| l.contains("Version:"))
        .collect();
    let updated_count = diff_lines(&pre_versions, &post_versions).len();

    if fail_count > 0 && updated_count == 0 {
        return UpdateStepResult {
            name: "claude",
            status: StepStatus::Fail(format!("{fail_count} plugin(s) failed")),
        };
    }

    let detail = if updated_count > 0 {
        format!("{updated_count} plugin(s) updated")
    } else {
        "no changes".to_string()
    };
    UpdateStepResult {
        name: "claude",
        status: StepStatus::Ok(detail),
    }
}

#[cfg(not(tarpaulin_include))]
fn update_npm(config: &ClaudeUpdateConfig) -> UpdateStepResult {
    if !has_cmd("npm") {
        return skip_result("npm", "npm not installed");
    }

    let pre = capture("npm", &["list", "-g", "--depth=0"]);

    let mut fail_count = 0usize;
    for pkg in &config.npm_globals {
        let exit = run_cmd("npm", &["install", "-g", pkg.as_str()]);
        if exit != 0 {
            warn!("npm global install failed for {pkg}");
            fail_count += 1;
        }
    }

    let post = capture("npm", &["list", "-g", "--depth=0"]);
    let npm_diff = diff_lines(&pre, &post);
    let count = npm_diff.len();

    if fail_count > 0 && count == 0 {
        return UpdateStepResult {
            name: "npm",
            status: StepStatus::Fail(format!("{fail_count} package(s) failed")),
        };
    }

    let detail = if count > 0 {
        let names: Vec<String> = npm_diff
            .iter()
            .map(|l| {
                l.trim_start_matches(|c: char| "├└─ ".contains(c))
                    .to_string()
            })
            .collect();
        format!("{count} package(s) ({})", names.join(", "))
    } else {
        "no changes".to_string()
    };
    UpdateStepResult {
        name: "npm",
        status: StepStatus::Ok(detail),
    }
}

#[cfg(not(tarpaulin_include))]
fn update_apt() -> UpdateStepResult {
    if !has_cmd("apt-get") {
        return skip_result(
            "apt",
            if std::env::consts::OS == "linux" {
                "apt not available"
            } else {
                "not applicable"
            },
        );
    }

    let pre = capture("dpkg-query", &["-W", "-f=${Package} ${Version}\n"]);

    let update_exit = Command::new("sudo")
        .args(["--non-interactive", "apt-get", "update", "-y"])
        .envs([
            ("DEBIAN_FRONTEND", "noninteractive"),
            ("NEEDRESTART_MODE", "a"),
        ])
        .status()
        .map(|s| s.code().unwrap_or(1))
        .unwrap_or(1);
    if update_exit != 0 {
        return fail_result("apt", update_exit);
    }

    let exit = Command::new("sudo")
        .args(["--non-interactive", "apt-get", "dist-upgrade", "-y"])
        .envs([
            ("DEBIAN_FRONTEND", "noninteractive"),
            ("NEEDRESTART_MODE", "a"),
        ])
        .status()
        .map(|s| s.code().unwrap_or(1))
        .unwrap_or(1);

    if exit != 0 {
        return fail_result("apt", exit);
    }

    let _ = Command::new("sudo")
        .args(["--non-interactive", "apt-get", "autoremove", "-y"])
        .envs([
            ("DEBIAN_FRONTEND", "noninteractive"),
            ("NEEDRESTART_MODE", "a"),
        ])
        .status();

    let post = capture("dpkg-query", &["-W", "-f=${Package} ${Version}\n"]);
    let apt_diff = diff_lines(&pre, &post);
    let count = apt_diff.len();

    let mut detail = if count > 0 {
        format!("{count} package(s) ({})", apt_diff.join(", "))
    } else {
        "no changes".to_string()
    };

    if Path::new("/var/run/reboot-required").exists() {
        detail.push_str(" — reboot required");
    }

    UpdateStepResult {
        name: "apt",
        status: StepStatus::Ok(detail),
    }
}

#[cfg(not(tarpaulin_include))]
fn update_snap(has_snap: bool) -> UpdateStepResult {
    if !has_snap || std::env::consts::OS != "linux" {
        return skip_result("snap", "not applicable");
    }
    if !has_cmd("snap") {
        return skip_result("snap", "snap not installed");
    }

    let pre: Vec<String> = capture("snap", &["list", "--color=never"])
        .into_iter()
        .skip(1)
        .map(|l| l.split_whitespace().take(2).collect::<Vec<_>>().join(" "))
        .collect();

    let exit = Command::new("sudo")
        .args(["--non-interactive", "snap", "refresh"])
        .status()
        .map(|s| s.code().unwrap_or(1))
        .unwrap_or(1);

    if exit != 0 {
        return fail_result("snap", exit);
    }

    let post: Vec<String> = capture("snap", &["list", "--color=never"])
        .into_iter()
        .skip(1)
        .map(|l| l.split_whitespace().take(2).collect::<Vec<_>>().join(" "))
        .collect();

    let snap_diff = diff_lines(&pre, &post);
    let detail = if snap_diff.is_empty() {
        "no changes".to_string()
    } else {
        format!("{} package(s) ({})", snap_diff.len(), snap_diff.join(", "))
    };
    UpdateStepResult {
        name: "snap",
        status: StepStatus::Ok(detail),
    }
}

#[cfg(not(tarpaulin_include))]
fn update_pip() -> UpdateStepResult {
    let pyenv_shim = home_dir().join(".pyenv/shims/python3");
    let (python_cmd, pip_extra) =
        match select_python_cmd(&pyenv_shim, has_cmd("python3"), has_cmd("python")) {
            Some(r) => r,
            None => return skip_result("pip", "python not installed"),
        };

    let outdated: Vec<String> = Command::new(&python_cmd)
        .args(pip_outdated_args(&pip_extra))
        .output()
        .ok()
        .filter(|o| o.status.success())
        .map(|o| {
            String::from_utf8_lossy(&o.stdout)
                .lines()
                .skip(2)
                .filter(|l| !l.is_empty())
                .map(|l| l.split_whitespace().next().unwrap_or("").to_string())
                .filter(|s| !s.is_empty())
                .collect()
        })
        .unwrap_or_default();

    if outdated.is_empty() {
        return UpdateStepResult {
            name: "pip",
            status: StepStatus::Ok("no changes".to_string()),
        };
    }

    let mut upgrade_args = vec!["-m", "pip", "install", "--upgrade"];
    upgrade_args.extend_from_slice(&pip_extra);
    let pkg_refs: Vec<&str> = outdated.iter().map(String::as_str).collect();
    upgrade_args.extend_from_slice(&pkg_refs);
    let exit = run_cmd(&python_cmd, &upgrade_args);

    let detail = format!("{} package(s) ({})", outdated.len(), outdated.join(", "));
    UpdateStepResult {
        name: "pip",
        status: if exit == 0 {
            StepStatus::Ok(detail)
        } else {
            StepStatus::Fail(format!("exit {exit}"))
        },
    }
}

#[cfg(not(tarpaulin_include))]
fn update_rust() -> UpdateStepResult {
    let rustup_in_cargo = home_dir().join(".cargo/bin/rustup");
    let rustup_cmd = if rustup_in_cargo.exists() {
        rustup_in_cargo.to_string_lossy().to_string()
    } else if has_cmd("rustup") {
        "rustup".to_string()
    } else {
        return skip_result("rust", "rustup not installed");
    };

    let pre = capture(&rustup_cmd, &["toolchain", "list"]);
    let exit = run_cmd(&rustup_cmd, &["update"]);
    if exit != 0 {
        return fail_result("rust", exit);
    }
    let post = capture(&rustup_cmd, &["toolchain", "list"]);
    let changed = diff_lines(&pre, &post);

    let cargo_bin = home_dir().join(".cargo/bin/cargo");
    let cargo_cmd = if cargo_bin.exists() {
        cargo_bin.to_string_lossy().to_string()
    } else {
        "cargo".to_string()
    };
    let nextest_bin = home_dir().join(".cargo/bin/cargo-nextest");
    if nextest_bin.exists() || has_cmd("cargo-nextest") {
        let _ = run_cmd(&cargo_cmd, &["install", "cargo-nextest", "--locked"]);
    }

    let detail = if changed.is_empty() {
        "no changes".to_string()
    } else {
        format!("{} toolchain(s) updated", changed.len())
    };
    UpdateStepResult {
        name: "rust",
        status: StepStatus::Ok(detail),
    }
}

#[cfg(not(tarpaulin_include))]
fn update_git_repo(name: &'static str, dir: &Path) -> UpdateStepResult {
    if !dir.exists() {
        return skip_result(name, "directory not found");
    }

    let old_sha = Command::new("git")
        .args(["-C", &dir.to_string_lossy(), "rev-parse", "HEAD"])
        .output()
        .ok()
        .map(|o| String::from_utf8_lossy(&o.stdout).trim().to_string())
        .unwrap_or_default();

    let exit = run_cmd("git", &["-C", &dir.to_string_lossy(), "pull"]);
    if exit != 0 {
        return fail_result(name, exit);
    }

    let count = if old_sha.is_empty() {
        0
    } else {
        git_commit_count(dir, &old_sha)
    };
    let detail = if count > 0 {
        format!("{count} commit(s)")
    } else {
        "no changes".to_string()
    };
    UpdateStepResult {
        name,
        status: StepStatus::Ok(detail),
    }
}

#[cfg(not(tarpaulin_include))]
fn update_gems() -> UpdateStepResult {
    let rbenv_shim = home_dir().join(".rbenv/shims/gem");
    let (gem_cmd, gem_extra) = match select_gem_cmd(&rbenv_shim, has_cmd("gem")) {
        Some(r) => r,
        None => return skip_result("gems", "gem not installed"),
    };

    let pre = capture(&gem_cmd, &["list"]);
    let mut update_args = vec!["update"];
    update_args.extend_from_slice(&gem_extra);
    let exit = run_cmd(&gem_cmd, &update_args);
    if exit != 0 {
        return fail_result("gems", exit);
    }
    let post = capture(&gem_cmd, &["list"]);
    let gem_diff = diff_lines(&pre, &post);
    let detail = if gem_diff.is_empty() {
        "no changes".to_string()
    } else {
        format!("{} gem(s) ({})", gem_diff.len(), gem_diff.join(", "))
    };
    UpdateStepResult {
        name: "gems",
        status: StepStatus::Ok(detail),
    }
}

#[cfg(not(tarpaulin_include))]
fn update_cheatsh() -> UpdateStepResult {
    let cht = home_dir().join("bin/cht.sh");
    if !cht.exists() {
        return skip_result("cheat.sh", "~/bin/cht.sh not found");
    }
    if !has_cmd("curl") {
        return skip_result("cheat.sh", "curl not installed");
    }

    // cht.sh does not publish checksums for its self-updating script; HTTPS is the
    // only available verification. Mirrors the dotfiles shell script approach.
    let exit = Command::new("curl")
        .args([
            "-fsSL",
            "https://cht.sh/:cht.sh",
            "-o",
            &cht.to_string_lossy(),
        ])
        .status()
        .map(|s| s.code().unwrap_or(1))
        .unwrap_or(1);

    if exit != 0 {
        return fail_result("cheat.sh", exit);
    }
    let _ = run_cmd("chmod", &["+x", &cht.to_string_lossy()]);
    UpdateStepResult {
        name: "cheat.sh",
        status: StepStatus::Ok("updated".to_string()),
    }
}

// ── Summary ───────────────────────────────────────────────────────────────────

fn print_summary<W: Write>(
    w: &mut W,
    results: &[UpdateStepResult],
    log_path: Option<&Path>,
) -> anyhow::Result<()> {
    let by_name: HashMap<&str, &UpdateStepResult> = results.iter().map(|r| (r.name, r)).collect();

    let timestamp = format_timestamp();
    let header = format!("=== Update Summary — {timestamp} ===\n\n");

    let mut body = String::new();
    let (mut ok, mut fail, mut skip, mut warn_count) = (0u32, 0u32, 0u32, 0u32);

    for &section in SECTION_ORDER {
        if let Some(result) = by_name.get(section) {
            match &result.status {
                StepStatus::Ok(_) => ok += 1,
                StepStatus::Fail(_) => fail += 1,
                StepStatus::Skip(_) => skip += 1,
                StepStatus::Warn(_) => warn_count += 1,
            }
            body.push_str(&format!(
                "{} {:<16} {}\n",
                result.status.tag(),
                section,
                result.status.detail()
            ));
        }
    }

    let total = ok + fail + skip + warn_count;
    let count_line = format!(
        "\n{total} sections: {ok} OK, {fail} failed, {warn_count} warnings, {skip} skipped\n"
    );

    write!(w, "{header}{body}{count_line}")?;

    if let Some(path) = log_path {
        let log_entry = format!(
            "────────────────────────────────────────────────────────\n{header}{body}{count_line}"
        );
        match std::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(path)
            .and_then(|mut f| f.write_all(log_entry.as_bytes()))
        {
            Ok(()) => writeln!(w, "Log appended: {}", path.display())?,
            Err(e) => warn!("Could not write to {}: {e}", path.display()),
        }
    }

    Ok(())
}

// ── EtchCommand impl ──────────────────────────────────────────────────────────

impl EtchCommand for Update {
    #[cfg(not(tarpaulin_include))]
    fn execute(&self, runtime: &Runtime) -> anyhow::Result<()> {
        self.validate_categories()?;
        let update_cfg = &runtime.config.update;
        let vars = &runtime.config.variables;
        let home = home_dir();

        let log_path = update_cfg
            .log_path
            .as_deref()
            .map(expand_tilde)
            .unwrap_or_else(|| home.join(".etch-update.log"));

        let has_snap = vars.get("has_snap").map(|v| v == "true").unwrap_or(false);
        let dotfiles_dir = vars.get("dotfiles_dir").map(|s| expand_tilde(s));

        let mut results: Vec<UpdateStepResult> = Vec::new();

        // git-tools (pull repos before package updates)
        if self.should_run("git-tools") {
            if let Some(ref gt) = update_cfg.git_tools {
                info!("Pulling git repos...");
                if gt.ai_config.is_some() {
                    let ai_dir = dotfiles_dir
                        .as_ref()
                        .and_then(|d| d.parent().map(|p| p.join("ai-config")));
                    results.push(match ai_dir {
                        Some(dir) => update_git_repo("ai-config", &dir),
                        None => skip_result("ai-config", "dotfiles_dir not set in variables"),
                    });
                }
                if gt.dotfiles.is_some() {
                    results.push(match &dotfiles_dir {
                        Some(dir) => update_git_repo("dotfiles", dir),
                        None => skip_result("dotfiles", "dotfiles_dir not set in variables"),
                    });
                }
                if gt.oh_my_zsh.unwrap_or(false) {
                    results.push(update_git_repo("oh-my-zsh", &home.join(".oh-my-zsh")));
                }
                if gt.tpm.unwrap_or(false) {
                    results.push(update_git_repo("tpm", &home.join(".tmux/plugins/tpm")));
                }
                if gt.tfenv.unwrap_or(false) {
                    results.push(update_git_repo("tfenv", &home.join(".tfenv")));
                }
            } else {
                info!("No git_tools config in etch.yaml — skipping");
            }
        }

        // brew
        if self.should_run("brew") {
            info!("Updating Homebrew...");
            results.push(update_brew());
        }

        // mas (macOS only — skips on Linux automatically)
        if self.should_run("mas") {
            results.push(update_mas());
        }

        // claude plugins + npm globals
        if self.should_run("claude") {
            match &update_cfg.claude {
                Some(claude_cfg) => {
                    info!("Updating Claude plugins...");
                    results.push(update_claude(claude_cfg));
                    if !claude_cfg.npm_globals.is_empty() {
                        results.push(update_npm(claude_cfg));
                    }
                }
                None => {
                    results.push(skip_result("claude", "no claude config in etch.yaml"));
                }
            }
        }

        // rust
        if self.should_run("rust") {
            if vars.get("has_rust").map(|v| v == "true").unwrap_or(true) {
                info!("Updating Rust toolchain...");
                results.push(update_rust());
            } else {
                results.push(skip_result("rust", "has_rust=false"));
            }
        }

        // packages: apt + snap (Linux)
        if self.should_run("packages") {
            results.push(update_apt());
            results.push(update_snap(has_snap));
        }

        // softwareupdate (macOS — skips on Linux automatically)
        if self.should_run("system") {
            results.push(update_softwareupdate());
        }

        // pip
        if self.should_run("pip") {
            if vars
                .get("has_devtools")
                .map(|v| v == "true")
                .unwrap_or(true)
            {
                results.push(update_pip());
            } else {
                results.push(skip_result("pip", "has_devtools=false"));
            }
        }

        // gems
        if self.should_run("gems") {
            results.push(update_gems());
        }

        // cheat.sh
        if self.should_run("cheatsh") {
            results.push(update_cheatsh());
        }

        let mut stdout = std::io::stdout();
        print_summary(&mut stdout, &results, Some(&log_path))?;

        Ok(())
    }

    #[cfg(tarpaulin_include)]
    fn execute(&self, _runtime: &Runtime) -> anyhow::Result<()> {
        unreachable!()
    }
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_claude_plugin_tokens_extracts_tokens() {
        let output = "❯ superpowers@official\n❯ context7@upstash\nsome other line\n";
        let tokens = parse_claude_plugin_tokens(output);
        assert_eq!(tokens, vec!["superpowers@official", "context7@upstash"]);
    }

    #[test]
    fn parse_claude_plugin_tokens_empty_output() {
        assert!(parse_claude_plugin_tokens("").is_empty());
    }

    #[test]
    fn diff_lines_returns_new_and_changed_entries() {
        let pre = vec!["a 1.0".to_string(), "b 2.0".to_string()];
        let post = vec![
            "a 1.1".to_string(),
            "b 2.0".to_string(),
            "c 3.0".to_string(),
        ];
        let diff = diff_lines(&pre, &post);
        assert!(diff.contains(&"a 1.1".to_string()), "expected updated a");
        assert!(diff.contains(&"c 3.0".to_string()), "expected new c");
        assert!(
            !diff.contains(&"b 2.0".to_string()),
            "unchanged b should not be in diff"
        );
    }

    #[test]
    fn diff_lines_empty_when_no_changes() {
        let pre = vec!["a 1.0".to_string()];
        let post = vec!["a 1.0".to_string()];
        assert!(diff_lines(&pre, &post).is_empty());
    }

    #[test]
    fn diff_lines_empty_input_produces_all_post() {
        let pre: Vec<String> = vec![];
        let post = vec!["a 1.0".to_string(), "b 2.0".to_string()];
        let diff = diff_lines(&pre, &post);
        assert_eq!(diff.len(), 2);
    }

    #[test]
    fn print_summary_formats_ok_result() {
        let results = vec![UpdateStepResult {
            name: "brew",
            status: StepStatus::Ok("3 formulae".to_string()),
        }];
        let mut buf = Vec::<u8>::new();
        print_summary(&mut buf, &results, None).unwrap();
        let out = String::from_utf8(buf).unwrap();
        assert!(out.contains("[OK]"), "expected [OK] in: {out}");
        assert!(out.contains("brew"), "expected 'brew' in: {out}");
        assert!(out.contains("3 formulae"), "expected detail in: {out}");
    }

    #[test]
    fn print_summary_formats_fail_result() {
        let results = vec![UpdateStepResult {
            name: "apt",
            status: StepStatus::Fail("exit 1".to_string()),
        }];
        let mut buf = Vec::<u8>::new();
        print_summary(&mut buf, &results, None).unwrap();
        let out = String::from_utf8(buf).unwrap();
        assert!(out.contains("[FAIL]"), "expected [FAIL] in: {out}");
        assert!(out.contains("exit 1"), "expected detail in: {out}");
    }

    #[test]
    fn print_summary_formats_skip_result() {
        let results = vec![UpdateStepResult {
            name: "mas",
            status: StepStatus::Skip("not applicable".to_string()),
        }];
        let mut buf = Vec::<u8>::new();
        print_summary(&mut buf, &results, None).unwrap();
        let out = String::from_utf8(buf).unwrap();
        assert!(out.contains("[SKIP]"), "expected [SKIP] in: {out}");
    }

    #[test]
    fn print_summary_counts_correctly() {
        let results = vec![
            UpdateStepResult {
                name: "brew",
                status: StepStatus::Ok("no changes".to_string()),
            },
            UpdateStepResult {
                name: "apt",
                status: StepStatus::Fail("exit 1".to_string()),
            },
            UpdateStepResult {
                name: "pip",
                status: StepStatus::Skip("not installed".to_string()),
            },
            UpdateStepResult {
                name: "rust",
                status: StepStatus::Warn("advisory".to_string()),
            },
        ];
        let mut buf = Vec::<u8>::new();
        print_summary(&mut buf, &results, None).unwrap();
        let out = String::from_utf8(buf).unwrap();
        assert!(
            out.contains("4 sections"),
            "expected section count in: {out}"
        );
        assert!(out.contains("1 OK"), "expected 1 OK in: {out}");
        assert!(out.contains("1 failed"), "expected 1 failed in: {out}");
        assert!(out.contains("1 warnings"), "expected 1 warnings in: {out}");
        assert!(out.contains("1 skipped"), "expected 1 skipped in: {out}");
    }

    #[test]
    fn print_summary_follows_section_order() {
        // rust appears before brew in input — brew should still come first in output
        let results = vec![
            UpdateStepResult {
                name: "rust",
                status: StepStatus::Ok("no changes".to_string()),
            },
            UpdateStepResult {
                name: "brew",
                status: StepStatus::Ok("1 formula".to_string()),
            },
        ];
        let mut buf = Vec::<u8>::new();
        print_summary(&mut buf, &results, None).unwrap();
        let out = String::from_utf8(buf).unwrap();
        let brew_pos = out.find("brew").expect("brew not found");
        let rust_pos = out.find("rust").expect("rust not found");
        assert!(brew_pos < rust_pos, "brew should appear before rust");
    }

    #[test]
    fn print_summary_omits_unreported_sections() {
        let results = vec![UpdateStepResult {
            name: "brew",
            status: StepStatus::Ok("no changes".to_string()),
        }];
        let mut buf = Vec::<u8>::new();
        print_summary(&mut buf, &results, None).unwrap();
        let out = String::from_utf8(buf).unwrap();
        assert!(!out.contains("apt"), "absent sections should not appear");
        assert!(!out.contains("rust"), "absent sections should not appear");
    }

    #[test]
    fn print_summary_appends_to_log_file() {
        let tmp = tempfile::tempdir().unwrap();
        let log_path = tmp.path().join("test.log");
        let results = vec![UpdateStepResult {
            name: "brew",
            status: StepStatus::Ok("no changes".to_string()),
        }];
        let mut buf = Vec::<u8>::new();
        print_summary(&mut buf, &results, Some(&log_path)).unwrap();
        // Log file must exist and contain summary
        let log_content = std::fs::read_to_string(&log_path).unwrap();
        assert!(log_content.contains("[OK]"), "log should contain summary");
        assert!(
            log_content.contains("brew"),
            "log should contain section name"
        );
        // Second call appends
        let mut buf2 = Vec::<u8>::new();
        print_summary(&mut buf2, &results, Some(&log_path)).unwrap();
        let log_content2 = std::fs::read_to_string(&log_path).unwrap();
        let occurrences = log_content2.matches("[OK]").count();
        assert!(occurrences >= 2, "log should have two appended entries");
    }

    #[test]
    fn expand_tilde_replaces_home() {
        let home = home_dir();
        let expanded = expand_tilde("~/.etch-update.log");
        assert!(
            expanded.starts_with(&home),
            "expected home prefix in: {expanded:?}"
        );
        assert!(
            expanded.to_string_lossy().ends_with(".etch-update.log"),
            "expected filename preserved"
        );
    }

    #[test]
    fn expand_tilde_leaves_absolute_paths() {
        let path = expand_tilde("/tmp/test.log");
        assert_eq!(path, PathBuf::from("/tmp/test.log"));
    }

    // ── select_python_cmd ──────────────────────────────────────────────────────

    fn make_shim(dir: &tempfile::TempDir, rel: &str) -> PathBuf {
        let p = dir.path().join(rel);
        std::fs::create_dir_all(p.parent().unwrap()).unwrap();
        std::fs::write(&p, "").unwrap();
        p
    }

    #[test]
    fn select_python_prefers_pyenv_when_shim_exists() {
        let tmp = tempfile::tempdir().unwrap();
        let shim = make_shim(&tmp, ".pyenv/shims/python3");
        let (cmd, extra) = select_python_cmd(&shim, true, true).unwrap();
        assert_eq!(cmd, shim.to_string_lossy().as_ref());
        assert!(
            extra.is_empty(),
            "pyenv shim should require no extra pip flags"
        );
    }

    #[test]
    fn select_python_falls_back_to_python3_with_system_flags() {
        let absent = PathBuf::from("/nonexistent/shims/python3");
        let (cmd, extra) = select_python_cmd(&absent, true, false).unwrap();
        assert_eq!(cmd, "python3");
        assert!(extra.contains(&"--user"), "system python3 needs --user");
        assert!(
            extra.contains(&"--break-system-packages"),
            "system python3 needs --break-system-packages"
        );
    }

    #[test]
    fn select_python_falls_back_to_python_when_python3_absent() {
        let absent = PathBuf::from("/nonexistent/shims/python3");
        let (cmd, extra) = select_python_cmd(&absent, false, true).unwrap();
        assert_eq!(cmd, "python");
        assert!(extra.contains(&"--user"));
        assert!(extra.contains(&"--break-system-packages"));
    }

    #[test]
    fn select_python_returns_none_when_nothing_installed() {
        let absent = PathBuf::from("/nonexistent/shims/python3");
        assert!(select_python_cmd(&absent, false, false).is_none());
    }

    #[test]
    fn select_python_ignores_system_fallbacks_when_pyenv_shim_exists() {
        let tmp = tempfile::tempdir().unwrap();
        let shim = make_shim(&tmp, ".pyenv/shims/python3");
        // has_python3=false, has_python=false — shim takes priority anyway
        let result = select_python_cmd(&shim, false, false);
        assert!(result.is_some(), "shim existence is sufficient");
        assert!(result.unwrap().1.is_empty());
    }

    // ── select_gem_cmd ────────────────────────────────────────────────────────

    #[test]
    fn select_gem_prefers_rbenv_when_shim_exists() {
        let tmp = tempfile::tempdir().unwrap();
        let shim = make_shim(&tmp, ".rbenv/shims/gem");
        let (cmd, extra) = select_gem_cmd(&shim, true).unwrap();
        assert_eq!(cmd, shim.to_string_lossy().as_ref());
        assert!(
            extra.is_empty(),
            "rbenv shim should require no extra gem flags"
        );
    }

    #[test]
    fn select_gem_falls_back_to_system_gem_with_user_install() {
        let absent = PathBuf::from("/nonexistent/rbenv/shims/gem");
        let (cmd, extra) = select_gem_cmd(&absent, true).unwrap();
        assert_eq!(cmd, "gem");
        assert!(
            extra.contains(&"--user-install"),
            "system gem needs --user-install to avoid permission errors"
        );
    }

    #[test]
    fn select_gem_returns_none_when_no_gem() {
        let absent = PathBuf::from("/nonexistent/rbenv/shims/gem");
        assert!(select_gem_cmd(&absent, false).is_none());
    }

    #[test]
    fn select_gem_ignores_system_fallback_when_rbenv_shim_exists() {
        let tmp = tempfile::tempdir().unwrap();
        let shim = make_shim(&tmp, ".rbenv/shims/gem");
        let result = select_gem_cmd(&shim, false);
        assert!(result.is_some(), "shim existence is sufficient");
        assert!(result.unwrap().1.is_empty());
    }

    // ── pip_outdated_args ─────────────────────────────────────────────────────

    #[test]
    fn pip_outdated_args_adds_user_flag_for_system_python() {
        let extra = vec!["--user", "--break-system-packages"];
        let args = pip_outdated_args(&extra);
        assert!(
            args.contains(&"--user"),
            "system Python list must be scoped to user packages to avoid system-managed conflicts"
        );
        assert!(args.contains(&"--outdated"));
    }

    #[test]
    fn pip_outdated_args_no_user_flag_for_pyenv_python() {
        let extra: Vec<&str> = vec![];
        let args = pip_outdated_args(&extra);
        assert!(
            !args.contains(&"--user"),
            "pyenv Python should list all packages, not just user-installed"
        );
        assert!(args.contains(&"--outdated"));
    }

    #[test]
    fn pip_outdated_args_always_includes_format_columns() {
        assert!(pip_outdated_args(&[]).contains(&"--format=columns"));
        assert!(pip_outdated_args(&["--user"]).contains(&"--format=columns"));
    }

    #[test]
    fn should_run_all_when_no_filter() {
        let u = Update::default();
        for cat in VALID_CATEGORIES {
            assert!(u.should_run(cat), "expected {cat} to run with no filter");
        }
    }

    #[test]
    fn should_run_only_selected() {
        let u = Update {
            only: vec!["brew".to_string(), "rust".to_string()],
            ..Default::default()
        };
        assert!(u.should_run("brew"));
        assert!(u.should_run("rust"));
        assert!(!u.should_run("pip"));
        assert!(!u.should_run("system"));
        assert!(!u.should_run("gems"));
    }

    #[test]
    fn should_run_skips_excluded() {
        let u = Update {
            skip: vec!["pip".to_string(), "gems".to_string()],
            ..Default::default()
        };
        assert!(u.should_run("brew"));
        assert!(!u.should_run("pip"));
        assert!(!u.should_run("gems"));
    }

    #[test]
    fn validate_categories_accepts_empty() {
        assert!(Update::default().validate_categories().is_ok());
    }

    #[test]
    fn validate_categories_accepts_valid() {
        let u = Update {
            only: vec!["brew".to_string(), "rust".to_string()],
            ..Default::default()
        };
        assert!(u.validate_categories().is_ok());
    }

    #[test]
    fn validate_categories_rejects_unknown_in_only() {
        let u = Update {
            only: vec!["foobar".to_string()],
            ..Default::default()
        };
        let err = u.validate_categories().unwrap_err();
        assert!(err.to_string().contains("unknown category 'foobar'"));
        assert!(err.to_string().contains("valid:"));
    }

    #[test]
    fn validate_categories_rejects_mixed_valid_invalid() {
        let u = Update {
            only: vec!["brew".to_string(), "badname".to_string()],
            ..Default::default()
        };
        assert!(u.validate_categories().is_err());
    }

    #[test]
    fn validate_categories_rejects_unknown_in_skip() {
        let u = Update {
            skip: vec!["notreal".to_string()],
            ..Default::default()
        };
        assert!(u.validate_categories().is_err());
    }
}
