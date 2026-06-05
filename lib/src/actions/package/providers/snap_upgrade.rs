use crate::atoms::command::Exec;
use crate::contexts::Contexts;
use crate::steps::Step;

pub(crate) fn plan_from_output(
    output: &str,
    name: Option<&str>,
    _context: &Contexts,
) -> anyhow::Result<Vec<Step>> {
    // Parse snap names from `snap refresh --list` output.
    // Header line starts with "Name"; data lines: "name version rev tracking publisher notes"
    // When nothing needs updating, snap outputs "All snaps up to date." to stdout —
    // all data lines contain '/' (tracking channel: latest/stable, etc.); that message
    // and the header do not, so filtering on '/' cleanly distinguishes them.
    let snap_names: Vec<&str> = output
        .lines()
        .filter(|l| l.contains('/'))
        .filter_map(|l| l.split_whitespace().next())
        .collect();

    if snap_names.is_empty() {
        return Ok(vec![]);
    }

    if let Some(n) = name {
        if !snap_names.contains(&n) {
            return Ok(vec![]);
        }
        Ok(vec![Step {
            atom: Box::new(Exec {
                command: String::from("snap"),
                arguments: vec![String::from("refresh"), n.to_string()],
                streaming: true,
                ..Default::default()
            }),
            initializers: vec![],
            finalizers: vec![],
        }])
    } else {
        Ok(vec![Step {
            atom: Box::new(Exec {
                command: String::from("snap"),
                arguments: vec![String::from("refresh")],
                streaming: true,
                ..Default::default()
            }),
            initializers: vec![],
            finalizers: vec![],
        }])
    }
}

pub(crate) fn plan(name: Option<&str>, context: &Contexts) -> anyhow::Result<Vec<Step>> {
    let output = std::process::Command::new("snap")
        .args(["refresh", "--list"])
        .stderr(std::process::Stdio::null())
        .output()
        .map(|o| String::from_utf8_lossy(&o.stdout).to_string())
        .unwrap_or_default();
    plan_from_output(&output, name, context)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::contexts::Contexts;

    const SNAP_OUTPUT: &str = "\
Name     Version  Rev  Tracking       Publisher   Notes
code     1.87.2   174  latest/stable  vscode✓     -
htop     3.3.0    55   latest/stable  hisham✓     -
";

    #[test]
    fn empty_output_returns_no_steps() {
        let steps = plan_from_output("", None, &Contexts::default()).unwrap();
        assert!(steps.is_empty());
    }

    #[test]
    fn header_only_returns_no_steps() {
        let steps = plan_from_output(
            "Name     Version  Rev  Tracking  Publisher  Notes\n",
            None,
            &Contexts::default(),
        )
        .unwrap();
        assert!(steps.is_empty());
    }

    #[test]
    fn snaps_with_no_filter_returns_one_refresh_all_step() {
        let steps = plan_from_output(SNAP_OUTPUT, None, &Contexts::default()).unwrap();
        assert_eq!(1, steps.len());
        let display = steps[0].atom.to_string();
        assert!(display.contains("snap"), "expected snap: {display}");
        assert!(display.contains("refresh"), "expected refresh: {display}");
    }

    #[test]
    fn refresh_all_does_not_include_specific_name() {
        let steps = plan_from_output(SNAP_OUTPUT, None, &Contexts::default()).unwrap();
        let display = steps[0].atom.to_string();
        assert!(
            !display.contains("code") && !display.contains("htop"),
            "refresh-all should not name a snap: {display}"
        );
    }

    #[test]
    fn all_snaps_up_to_date_message_returns_no_steps() {
        let steps =
            plan_from_output("All snaps up to date.\n", None, &Contexts::default()).unwrap();
        assert!(
            steps.is_empty(),
            "expected no steps for 'all up to date' message"
        );
    }

    #[test]
    fn named_snap_in_list_returns_targeted_refresh() {
        let steps = plan_from_output(SNAP_OUTPUT, Some("code"), &Contexts::default()).unwrap();
        assert_eq!(1, steps.len());
        let display = steps[0].atom.to_string();
        assert!(display.contains("snap"), "expected snap: {display}");
        assert!(display.contains("refresh"), "expected refresh: {display}");
        assert!(display.contains("code"), "expected 'code': {display}");
    }

    #[test]
    fn named_snap_not_in_list_returns_no_steps() {
        let steps = plan_from_output(SNAP_OUTPUT, Some("firefox"), &Contexts::default()).unwrap();
        assert!(steps.is_empty());
    }
}
