pub mod install;
pub mod upgrade;

pub(crate) use install::ClaudeInstall;
pub(crate) use upgrade::ClaudeUpgrade;

/// Parse installed plugin tokens from `claude plugins list` stdout.
/// Returns full `name@marketplace` tokens from lines starting with `❯ `.
pub(crate) fn parse_plugin_list(output: &str) -> Vec<String> {
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

#[allow(dead_code)] // used by marketplace.rs (added in next commit)
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
