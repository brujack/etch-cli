use super::EtchCommand;
use crate::Runtime;
use etch_lib::rollback::{StashEntry, StashStore};
use std::path::{Path, PathBuf};

#[derive(clap::Args, Debug)]
pub(crate) struct Rollback {
    /// Restore latest stash for this path
    #[arg(long)]
    pub path: Option<PathBuf>,
    /// List all stashed paths with timestamps (default behavior)
    #[arg(long)]
    pub list: bool,
    /// Show diff of what would be restored; no write (requires --path)
    #[arg(long)]
    pub dry_run: bool,
    /// Restore all paths to their latest stash
    #[arg(long)]
    pub all: bool,
    /// Skip confirmation prompt for --all
    #[arg(long)]
    pub yes: bool,
}

/// Validate mutually exclusive arg combinations.
pub(crate) fn validate_args(args: &Rollback) -> anyhow::Result<()> {
    if args.all && args.dry_run {
        anyhow::bail!("--dry-run cannot be combined with --all");
    }
    Ok(())
}

/// Format the stash table as a String (extracted for unit testing).
pub(crate) fn render_list(entries: &[(PathBuf, Vec<StashEntry>)]) -> String {
    use std::fmt::Write as FmtWrite;
    let mut buf = String::new();
    writeln!(buf, "{:<50} {:<8} LATEST", "PATH", "STASHES").unwrap();
    for (path, stashes) in entries {
        let latest = &stashes[0]; // sorted newest-first by StashStore::list()
        writeln!(
            buf,
            "{:<50} {:<8} {}",
            path.display(),
            stashes.len(),
            latest.stashed_at.format("%Y-%m-%d %H:%M:%S UTC"),
        )
        .unwrap();
    }
    buf
}

pub(crate) fn expand_tilde(path: &Path) -> PathBuf {
    let s = path.to_string_lossy();
    if let Some(rest) = s.strip_prefix("~/") {
        if let Some(home) = dirs_next::home_dir() {
            return home.join(rest);
        }
    }
    path.to_path_buf()
}

impl EtchCommand for Rollback {
    #[cfg(not(tarpaulin_include))]
    fn execute(&self, _runtime: &Runtime) -> anyhow::Result<()> {
        use std::io::IsTerminal;

        if let Err(e) = validate_args(self) {
            eprintln!("error: {e}");
            std::process::exit(1);
        }

        let store = StashStore::new();

        // --path: restore or dry-run diff
        if let Some(raw_path) = &self.path {
            let path = expand_tilde(raw_path);
            if let Err(e) = store.restore(&path, self.dry_run) {
                eprintln!("error: {e}");
                std::process::exit(1);
            }
            return Ok(());
        }

        // --all: restore everything
        if self.all {
            if !std::io::stdin().is_terminal() && !self.yes {
                eprintln!("error: --all requires confirmation; pass --yes to skip prompt");
                std::process::exit(1);
            }
            let list = match store.list() {
                Ok(l) => l,
                Err(e) => {
                    eprintln!("error: {e}");
                    std::process::exit(1);
                }
            };

            if list.is_empty() {
                println!("No stashes found.");
                return Ok(());
            }

            println!("Will restore {} path(s) to their latest stash:", list.len());
            for (path, stashes) in &list {
                println!(
                    "  {} \u{2192} stash {}",
                    path.display(),
                    stashes[0].stashed_at.format("%Y-%m-%dT%H:%M:%SZ")
                );
            }

            if !self.yes {
                use std::io::Write;
                print!("Continue? [y/N] ");
                std::io::stdout().flush()?;
                let mut input = String::new();
                std::io::BufRead::read_line(&mut std::io::stdin().lock(), &mut input)?;
                if !input.trim().eq_ignore_ascii_case("y") {
                    println!("Aborted.");
                    return Ok(());
                }
            }

            for (path, _) in &list {
                if let Err(e) = store.restore(path, false) {
                    eprintln!("error restoring {}: {e}", path.display());
                }
            }
            return Ok(());
        }

        // Default: list
        let list = store.list()?;
        print!("{}", render_list(&list));

        Ok(())
    }

    #[cfg(tarpaulin_include)]
    fn execute(&self, _runtime: &Runtime) -> anyhow::Result<()> {
        unreachable!()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Utc;

    fn make_rollback(all: bool, dry_run: bool, path: Option<PathBuf>) -> Rollback {
        Rollback {
            path,
            list: false,
            dry_run,
            all,
            yes: false,
        }
    }

    fn make_entry(path: &str, ts: chrono::DateTime<chrono::Utc>) -> StashEntry {
        StashEntry {
            original_path: PathBuf::from(path),
            stashed_at: ts,
            apply_manifest: "m.yaml".into(),
            sha256: "abc".into(),
            stash_path: PathBuf::from("/tmp/s"),
            meta_path: PathBuf::from("/tmp/s.meta.yaml"),
        }
    }

    #[test]
    fn validate_all_dry_run_errors() {
        let args = make_rollback(true, true, None);
        assert!(validate_args(&args).is_err());
    }

    #[test]
    fn validate_path_dry_run_ok() {
        let args = make_rollback(false, true, Some(PathBuf::from("/tmp/f")));
        assert!(validate_args(&args).is_ok());
    }

    #[test]
    fn validate_no_args_ok() {
        let args = make_rollback(false, false, None);
        assert!(validate_args(&args).is_ok());
    }

    #[test]
    fn render_list_empty_shows_header_only() {
        let output = render_list(&[]);
        assert!(output.contains("PATH"), "header must be present");
        assert!(output.contains("STASHES"));
        assert!(output.contains("LATEST"));
    }

    #[test]
    fn render_list_shows_path_and_count() {
        let now = Utc::now();
        let earlier = now - chrono::Duration::seconds(100);
        let entries = vec![(
            PathBuf::from("/home/bruce/.zshrc"),
            vec![
                make_entry("/home/bruce/.zshrc", now),
                make_entry("/home/bruce/.zshrc", earlier),
            ],
        )];
        let output = render_list(&entries);
        assert!(output.contains(".zshrc"), "path must appear");
        assert!(output.contains('2'), "stash count must appear");
    }

    #[test]
    fn expand_tilde_replaces_prefix() {
        let home = dirs_next::home_dir().unwrap();
        let path = PathBuf::from("~/.zshrc");
        let expanded = expand_tilde(&path);
        assert_eq!(expanded, home.join(".zshrc"));
    }

    #[test]
    fn expand_tilde_noop_for_absolute() {
        let path = PathBuf::from("/etc/hosts");
        assert_eq!(expand_tilde(&path), path);
    }
}
