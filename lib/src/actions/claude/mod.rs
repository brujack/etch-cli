pub mod install;
pub mod upgrade;

#[allow(unused_imports)]
pub(crate) use install::ClaudeInstall;
#[allow(unused_imports)]
pub(crate) use upgrade::ClaudeUpgrade;

/// Parse installed plugin tokens from `claude plugins list` stdout.
/// Returns full `name@marketplace` tokens from lines starting with `❯ `.
#[allow(dead_code)]
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
}
