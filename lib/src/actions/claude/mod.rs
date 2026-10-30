pub mod install;
pub mod marketplace;
pub mod marketplace_remove;
pub mod plugin_update;
pub mod upgrade;

pub(crate) use install::ClaudeInstall;
pub(crate) use marketplace::ClaudeMarketplace;
pub(crate) use marketplace_remove::ClaudeMarketplaceRemove;
pub(crate) use plugin_update::ClaudePluginUpdate;
pub(crate) use upgrade::ClaudeUpgrade;

/// Parse installed plugin tokens from `claude plugins list` stdout.
/// Returns full `name@marketplace` tokens from lines starting with `❯ `, each
/// once in first-seen order: the list prints one entry per scope and per
/// project, so one plugin can appear hundreds of times.
pub(crate) fn parse_plugin_list(output: &str) -> Vec<String> {
    let mut seen = std::collections::HashSet::new();
    output
        .lines()
        .filter_map(|line| {
            let trimmed = line.trim();
            trimmed
                .strip_prefix('❯')
                .map(|rest| rest.trim().to_string())
                .filter(|s| !s.is_empty())
        })
        .filter(|token| seen.insert(token.clone()))
        .collect()
}

pub(crate) fn parse_marketplace_list(output: &str) -> Vec<String> {
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

#[cfg(test)]
mod tests {
    use super::*;
    use proptest::prelude::*;

    proptest! {
        #[test]
        fn parse_yields_each_listed_token_once_in_first_seen_order(
            picks in proptest::collection::vec(0usize..4, 0..40),
        ) {
            let names = ["a@m", "b@m", "c@n", "d@n"];
            let output: String = picks
                .iter()
                .map(|&i| format!("  ❯ {}\n    Scope: project\n\n", names[i]))
                .collect();

            let mut expected: Vec<&str> = Vec::new();
            for &i in &picks {
                if !expected.contains(&names[i]) {
                    expected.push(names[i]);
                }
            }

            prop_assert_eq!(parse_plugin_list(&output), expected);
        }
    }

    #[test]
    fn parse_returns_full_tokens_from_list_output() {
        let output = "❯ superpowers@claude-plugins-official\n❯ context7@upstash-context7\n";
        let tokens = parse_plugin_list(output);
        assert_eq!(
            tokens,
            vec![
                "superpowers@claude-plugins-official",
                "context7@upstash-context7"
            ]
        );
    }

    #[test]
    fn parse_lists_a_plugin_installed_in_several_scopes_once() {
        // `claude plugins list` prints one entry per scope and project, so the
        // same token repeats; each must come back once, in first-seen order.
        let output = "❯ caveman@caveman\n    Scope: project\n❯ foo@bar\n❯ caveman@caveman\n    Scope: user\n❯ foo@bar\n❯ caveman@caveman\n";
        let tokens = parse_plugin_list(output);
        assert_eq!(tokens, vec!["caveman@caveman", "foo@bar"]);
    }

    #[test]
    fn parse_returns_empty_for_empty_output() {
        assert!(parse_plugin_list("").is_empty());
    }

    #[test]
    fn parse_skips_non_matching_lines() {
        let output = "Some header\n  other line\n❯ foo@bar\n";
        let tokens = parse_plugin_list(output);
        assert_eq!(tokens, vec!["foo@bar"]);
    }

    #[test]
    fn parse_marketplace_list_extracts_names() {
        let output = "Configured marketplaces:\n\n  ❯ claude-plugins-official\n    Source: GitHub (anthropics/claude-plugins-official)\n\n  ❯ caveman\n    Source: Git (https://github.com/juliusbrussee/caveman.git)\n";
        let names = parse_marketplace_list(output);
        assert_eq!(names, vec!["claude-plugins-official", "caveman"]);
    }

    #[test]
    fn parse_marketplace_list_empty_input_returns_empty() {
        assert!(parse_marketplace_list("").is_empty());
    }

    #[test]
    fn parse_marketplace_list_skips_source_lines() {
        let output = "  ❯ foo\n    Source: GitHub (bar/baz)\n";
        let names = parse_marketplace_list(output);
        assert_eq!(names, vec!["foo"]);
    }
}
