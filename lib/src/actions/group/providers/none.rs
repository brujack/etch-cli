use super::GroupProvider;
use crate::actions::group::GroupVariant;
use crate::contexts::Contexts;
use crate::steps::Step;
use serde::{Deserialize, Serialize};
use tracing::warn;

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct NoneGroupProvider {}

impl GroupProvider for NoneGroupProvider {
    fn add_group(&self, _group: &GroupVariant, _contexts: &Contexts) -> Vec<Step> {
        warn!("This system does not have a provider for groups");
        vec![]
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::actions::group::GroupVariant;
    use crate::config::Config;
    use crate::contexts::build_contexts;

    #[test]
    fn add_group_returns_empty_steps() {
        let provider = NoneGroupProvider {};
        let group = GroupVariant::default();
        let contexts = build_contexts(&Config::default());
        let steps = provider.add_group(&group, &contexts);
        assert!(steps.is_empty());
    }
}
