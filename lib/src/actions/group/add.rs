use super::Group;
use super::GroupVariant;
use crate::actions::Action;
use crate::contexts::Contexts;
use crate::manifests::Manifest;
use crate::steps::Step;
use std::ops::Deref;

pub type GroupAdd = Group;

impl Action for GroupAdd {
    fn summarize(&self) -> String {
        format!("Creating group {}", self.group_name)
    }

    fn plan(&self, _manifest: &Manifest, contexts: &Contexts) -> anyhow::Result<Vec<Step>> {
        let variant: GroupVariant = self.into();
        let box_provider = variant.provider.clone().get_provider();
        let provider = box_provider.deref();

        let mut atoms: Vec<Step> = vec![];

        atoms.append(&mut provider.add_group(&variant, contexts));

        Ok(atoms)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::actions::Action;
    use crate::config::Config;
    use crate::contexts::build_contexts;

    #[test]
    fn plan_returns_ok() {
        let action = GroupAdd {
            group_name: "testgroup".to_string(),
            ..Default::default()
        };
        let manifest = crate::test_helpers::make_manifest(std::path::Path::new("/tmp"));
        let contexts = build_contexts(&Config::default());
        assert!(action.plan(&manifest, &contexts).is_ok());
    }
}
