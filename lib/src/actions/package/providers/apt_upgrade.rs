use crate::atoms::command::Exec;
use crate::contexts::Contexts;
use crate::steps::Step;
use crate::utilities;

pub(crate) fn plan_from_output(
    output: &str,
    name: Option<&str>,
    list: Option<&[String]>,
    context: &Contexts,
) -> anyhow::Result<Vec<Step>> {
    // Parse upgradable package names from `apt list --upgradable` output.
    // Format: "pkg/suite version arch [upgradable from: old]"
    let upgradable: Vec<&str> = output
        .lines()
        .filter(|l| !l.starts_with("Listing...") && !l.trim().is_empty())
        .filter_map(|l| l.split('/').next())
        .collect();

    if upgradable.is_empty() {
        return Ok(vec![]);
    }

    let pkgs: Vec<String> = if let Some(n) = name {
        if upgradable.contains(&n) {
            vec![n.to_string()]
        } else {
            return Ok(vec![]);
        }
    } else if let Some(lst) = list {
        let filtered: Vec<String> = lst
            .iter()
            .filter(|p| upgradable.contains(&p.as_str()))
            .cloned()
            .collect();
        if filtered.is_empty() {
            return Ok(vec![]);
        }
        filtered
    } else {
        vec![] // upgrade all
    };

    let privilege_provider =
        utilities::get_privilege_provider(context).unwrap_or_else(|| "sudo".to_string());
    let env = vec![(
        String::from("DEBIAN_FRONTEND"),
        String::from("noninteractive"),
    )];

    let update_step = Step {
        atom: Box::new(Exec {
            command: String::from("apt-get"),
            arguments: vec![String::from("update")],
            environment: env.clone(),
            privileged: true,
            privilege_provider: privilege_provider.clone(),
            ..Default::default()
        }),
        initializers: vec![],
        finalizers: vec![],
    };

    let upgrade_args = if pkgs.is_empty() {
        vec![String::from("upgrade"), String::from("-y")]
    } else {
        let mut args = vec![
            String::from("install"),
            String::from("--only-upgrade"),
            String::from("-y"),
        ];
        args.extend(pkgs);
        args
    };

    let upgrade_step = Step {
        atom: Box::new(Exec {
            command: String::from("apt-get"),
            arguments: upgrade_args,
            environment: env,
            privileged: true,
            privilege_provider,
            streaming: true,
            ..Default::default()
        }),
        initializers: vec![],
        finalizers: vec![],
    };

    Ok(vec![update_step, upgrade_step])
}

pub(crate) fn plan(
    name: Option<&str>,
    list: Option<&[String]>,
    context: &Contexts,
) -> anyhow::Result<Vec<Step>> {
    let output = std::process::Command::new("apt")
        .args(["list", "--upgradable"])
        .stderr(std::process::Stdio::null())
        .output()
        .map(|o| String::from_utf8_lossy(&o.stdout).to_string())
        .unwrap_or_default();
    plan_from_output(&output, name, list, context)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::contexts::Contexts;

    const APT_OUTPUT: &str = "\
Listing... Done
git/jammy-updates 1:2.34.1-1ubuntu1.12 amd64 [upgradable from: 1:2.34.1-1ubuntu1.11]
curl/jammy-updates 7.81.0-1ubuntu1.16 amd64 [upgradable from: 7.81.0-1ubuntu1.15]
";

    #[test]
    fn empty_output_returns_no_steps() {
        let steps = plan_from_output("", None, None, &Contexts::default()).unwrap();
        assert!(steps.is_empty());
    }

    #[test]
    fn listing_only_line_returns_no_steps() {
        let steps =
            plan_from_output("Listing... Done\n", None, None, &Contexts::default()).unwrap();
        assert!(steps.is_empty());
    }

    #[test]
    fn upgradable_packages_with_no_filter_returns_two_steps() {
        let steps = plan_from_output(APT_OUTPUT, None, None, &Contexts::default()).unwrap();
        assert_eq!(2, steps.len());
        let update = steps[0].atom.to_string();
        let upgrade = steps[1].atom.to_string();
        assert!(update.contains("apt-get"), "step 0: {update}");
        assert!(update.contains("update"), "step 0: {update}");
        assert!(upgrade.contains("apt-get"), "step 1: {upgrade}");
        assert!(upgrade.contains("upgrade"), "step 1: {upgrade}");
    }

    #[test]
    fn upgradable_packages_with_no_filter_uses_upgrade_not_only_upgrade() {
        let steps = plan_from_output(APT_OUTPUT, None, None, &Contexts::default()).unwrap();
        let upgrade = steps[1].atom.to_string();
        assert!(
            !upgrade.contains("--only-upgrade"),
            "upgrade-all should not use --only-upgrade: {upgrade}"
        );
    }

    #[test]
    fn named_package_in_upgradable_list_returns_two_steps_with_only_upgrade() {
        let steps = plan_from_output(APT_OUTPUT, Some("git"), None, &Contexts::default()).unwrap();
        assert_eq!(2, steps.len());
        let upgrade = steps[1].atom.to_string();
        assert!(
            upgrade.contains("--only-upgrade"),
            "expected --only-upgrade: {upgrade}"
        );
        assert!(upgrade.contains("git"), "expected 'git': {upgrade}");
    }

    #[test]
    fn named_package_not_in_upgradable_list_returns_no_steps() {
        let steps = plan_from_output(APT_OUTPUT, Some("vim"), None, &Contexts::default()).unwrap();
        assert!(steps.is_empty());
    }

    #[test]
    fn list_partially_upgradable_returns_only_upgradable_packages() {
        let list = vec!["git".to_string(), "vim".to_string()];
        let steps = plan_from_output(APT_OUTPUT, None, Some(&list), &Contexts::default()).unwrap();
        assert_eq!(2, steps.len());
        let upgrade = steps[1].atom.to_string();
        assert!(upgrade.contains("git"), "expected 'git': {upgrade}");
        assert!(!upgrade.contains("vim"), "vim not upgradable: {upgrade}");
    }

    #[test]
    fn list_none_upgradable_returns_no_steps() {
        let list = vec!["vim".to_string(), "nano".to_string()];
        let steps = plan_from_output(APT_OUTPUT, None, Some(&list), &Contexts::default()).unwrap();
        assert!(steps.is_empty());
    }

    #[test]
    fn both_packages_in_list_upgradable_returns_both() {
        let list = vec!["git".to_string(), "curl".to_string()];
        let steps = plan_from_output(APT_OUTPUT, None, Some(&list), &Contexts::default()).unwrap();
        assert_eq!(2, steps.len());
        let upgrade = steps[1].atom.to_string();
        assert!(upgrade.contains("git"), "expected 'git': {upgrade}");
        assert!(upgrade.contains("curl"), "expected 'curl': {upgrade}");
    }

    #[test]
    fn steps_are_privileged() {
        let steps = plan_from_output(APT_OUTPUT, None, None, &Contexts::default()).unwrap();
        for step in &steps {
            let display = step.atom.to_string();
            assert!(
                display.contains("privileged=true"),
                "expected privileged=true: {display}"
            );
        }
    }
}
