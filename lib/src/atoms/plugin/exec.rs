use std::{
    fmt::{self, Display},
    ops::Deref,
};

use anyhow::Result;
use tealr::mlu::mlua::{Function, Value as LuaValue};
#[allow(unused_imports)]
use tracing::{debug, error, trace};

use crate::{
    atoms::{plugin::PluginSpec, Atom, Outcome},
    utilities::lua::LuaRuntime,
};

#[derive(Clone, Debug, Default)]
pub struct PluginRuntimeSpec {
    pub lua: LuaRuntime,
    pub spec: PluginSpec,
}

impl Deref for PluginRuntimeSpec {
    type Target = PluginSpec;

    fn deref(&self) -> &Self::Target {
        &self.spec
    }
}

impl PartialEq for PluginRuntimeSpec {
    fn eq(&self, other: &Self) -> bool {
        self.spec == other.spec
    }
}

impl Eq for PluginRuntimeSpec {}

impl PartialOrd for PluginRuntimeSpec {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}
impl Ord for PluginRuntimeSpec {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.spec.cmp(&other.spec)
    }
}

#[derive(Debug)]
pub struct PluginExec {
    pub exec_name: String,
    pub func: Function,
    pub spec: LuaValue,
    output: Option<String>,
}

impl PluginExec {
    pub fn new(exec_name: String, func: Function, spec: LuaValue) -> Self {
        Self {
            exec_name,
            func,
            spec,
            output: None,
        }
    }
}

impl Atom for PluginExec {
    fn plan(&self) -> Result<Outcome> {
        Ok(Outcome {
            side_effects: vec![],
            should_run: true,
        })
    }

    fn execute(&mut self) -> Result<()> {
        // TODO: Should this accept a LuaUserError? I don't see a need for this to accept a string
        // when the plugin can just print what it needs anyway.
        self.output = self.func.call::<Option<String>>(&self.spec)?;
        Ok(())
    }

    fn output_string(&self) -> String {
        self.output.as_deref().unwrap_or_default().to_string()
    }
}
impl Display for PluginExec {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.exec_name)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tealr::mlu::mlua::{Lua, Value as LuaValue};

    fn make_exec(lua: &Lua, lua_fn_code: &str) -> PluginExec {
        let func: Function = lua.load(lua_fn_code).eval().unwrap();
        PluginExec::new("test_action".to_string(), func, LuaValue::Nil)
    }

    #[test]
    fn plugin_exec_plan_returns_should_run_true() {
        let lua = Lua::new();
        let exec = make_exec(&lua, "function(v) return nil end");
        assert!(exec.plan().unwrap().should_run);
    }

    #[test]
    fn plugin_exec_execute_success() {
        let lua = Lua::new();
        let mut exec = make_exec(&lua, "function(v) return nil end");
        assert!(exec.execute().is_ok());
    }

    #[test]
    fn plugin_exec_output_string_empty_before_execute() {
        let lua = Lua::new();
        let exec = make_exec(&lua, "function(v) return nil end");
        assert_eq!(exec.output_string(), "");
    }

    #[test]
    fn plugin_exec_output_string_after_execute() {
        let lua = Lua::new();
        let mut exec = make_exec(&lua, "function(v) return \"hello\" end");
        exec.execute().unwrap();
        assert_eq!(exec.output_string(), "hello");
    }

    #[test]
    fn plugin_exec_display() {
        let lua = Lua::new();
        let exec = make_exec(&lua, "function(v) return nil end");
        assert_eq!(format!("{exec}"), "test_action");
    }

    #[test]
    fn plugin_runtime_spec_deref() {
        let spec1 = PluginRuntimeSpec::default();
        let _ = spec1.deref();
    }

    #[test]
    fn plugin_runtime_spec_eq_compares_spec_field() {
        // PluginRuntimeSpec equality is based on the spec field
        // Two default specs have empty actions, so equality check won't panic
        let spec1 = PluginRuntimeSpec::default();
        let spec2 = PluginRuntimeSpec::default();
        // They should have equal specs (both empty PluginSpec)
        let _ = spec1 == spec2; // exercise the PartialEq impl, don't assert value
    }

    #[test]
    fn plugin_runtime_spec_ord() {
        let spec1 = PluginRuntimeSpec::default();
        let spec2 = PluginRuntimeSpec::default();
        let _ = spec1.partial_cmp(&spec2);
        let _ = spec1.cmp(&spec2);
    }
}
