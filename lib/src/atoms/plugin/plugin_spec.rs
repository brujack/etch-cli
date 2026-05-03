use std::{
    cmp::Ordering,
    collections::BTreeMap,
    fmt::{self, Display, Formatter},
    hash::{Hash, Hasher},
    ops::Deref,
    ptr::addr_of,
};

use tealr::{
    mlu::mlua::{Error as LuaError, FromLua, Function, Lua, Table, Value},
    ToTypename,
};

use crate::utilities::lua::LuaFunction;

#[derive(Debug, Clone, Default)]
pub struct ComparableLua(Lua);

impl Deref for ComparableLua {
    type Target = Lua;
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl Hash for ComparableLua {
    fn hash<H: Hasher>(&self, state: &mut H) {
        addr_of!(self.0).hash(state)
    }
}

impl PartialEq for ComparableLua {
    fn eq(&self, other: &Self) -> bool {
        addr_of!(self) == addr_of!(other)
    }
}

impl Eq for ComparableLua {}

impl PartialOrd for ComparableLua {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for ComparableLua {
    fn cmp(&self, other: &Self) -> Ordering {
        addr_of!(self).cmp(&addr_of!(other))
    }
}

#[derive(Debug, Clone)]
pub struct ComparableFunction(Function);

impl Deref for ComparableFunction {
    type Target = Function;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl Hash for ComparableFunction {
    fn hash<H: Hasher>(&self, state: &mut H) {
        // Hash based on pointer address or another stable identifier
        std::ptr::addr_of!(self.0).hash(state);
    }
}

impl PartialEq for ComparableFunction {
    fn eq(&self, other: &Self) -> bool {
        self.to_pointer() == other.to_pointer()
    }
}

impl Eq for ComparableFunction {}

impl PartialOrd for ComparableFunction {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for ComparableFunction {
    fn cmp(&self, other: &Self) -> Ordering {
        self.to_pointer().cmp(&other.to_pointer())
    }
}

#[derive(Clone, Debug, ToTypename, PartialEq, Eq, Hash, PartialOrd, Ord, Default)]
pub enum StringOrFunction {
    String(String),
    Function(ComparableFunction),
    #[default]
    Invalid,
}

#[derive(Clone, Debug, ToTypename, Default, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct PluginAction {
    pub plan: Option<LuaFunction>,
    pub exec: LuaFunction,
    pub is_privileged: bool,
}

#[derive(Clone, Debug, ToTypename, Default, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct PluginSpec {
    name: StringOrFunction,
    summary: Option<StringOrFunction>,
    pub actions: BTreeMap<String, PluginAction>,
    #[tealr(skip)]
    pub lua: ComparableLua,
}

impl PluginSpec {
    pub fn name(&self) -> String {
        match self.name {
            StringOrFunction::String(ref s) => s.clone(),
            StringOrFunction::Function(ref f) => f
                .call::<String>(Value::default())
                .unwrap_or(String::from("anonymous")),
            StringOrFunction::Invalid => String::from("anonymous"),
        }
    }

    pub fn summary(&self) -> String {
        match self.summary {
            Some(StringOrFunction::String(ref s)) => s.clone(),
            Some(StringOrFunction::Function(ref f)) => f
                .call::<String>(Value::default())
                .unwrap_or(String::from("anonymous")),
            _ => format!("Plugin {} completed.", self.name()),
        }
    }

    pub fn get_action(&self, name: &str) -> Option<&PluginAction> {
        self.actions.get(name)
    }

    pub fn exec_action(&self, action_name: &str, args: Value) -> Result<Value, LuaError> {
        self.get_action(action_name)
            .map(|action| action.exec.call(args))
            .unwrap_or_else(|| Err(LuaError::external("No action found")))
    }

    pub fn plan_action(&self, name: &str, args: Value) -> Option<Result<Value, LuaError>> {
        self.get_action(name)
            .and_then(|action| action.plan.as_ref())
            .map(|plan| plan.call(args))
    }
}

impl Display for PluginSpec {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(f, "plugin.{}", self.name())
    }
}

impl FromLua for PluginSpec {
    fn from_lua(value: Value, lua: &Lua) -> Result<Self, LuaError> {
        let Value::Table(table) = value else {
            return Err(LuaError::FromLuaConversionError {
                from: value.type_name(),
                to: String::from("PluginSpec"),
                message: Some("Expected a Lua table but got a different type".to_string()),
            });
        };

        let actions: BTreeMap<String, PluginAction> = table
            .get::<Table>("actions")?
            .pairs::<String, Table>()
            .flatten()
            .map(|(key, action)| {
                let action = PluginAction {
                    plan: action.get::<Function>("plan").map(LuaFunction).ok(),
                    exec: action.get::<Function>("exec").map(LuaFunction)?,
                    is_privileged: action.get("is_privileged").unwrap_or(false),
                };
                Ok((key, action))
            })
            .collect::<Result<_, LuaError>>()?;

        let name = table.get::<Value>("name").map(|n| match n {
            Value::String(s) => StringOrFunction::String(s.to_string_lossy()),
            Value::Function(f) => StringOrFunction::Function(ComparableFunction(f)),
            _ => StringOrFunction::Invalid,
        })?;

        let summary = table.get::<Value>("summary").ok().and_then(|s| match s {
            Value::String(s) => Some(StringOrFunction::String(s.to_string_lossy())),
            Value::Function(f) => Some(StringOrFunction::Function(ComparableFunction(f))),
            _ => None,
        });

        Ok(PluginSpec {
            name,
            summary,
            actions,
            lua: ComparableLua(lua.to_owned()),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tealr::mlu::mlua::Lua;

    fn make_plugin_spec(lua_code: &str) -> PluginSpec {
        let lua = Lua::new();
        let value: Value = lua.load(lua_code).eval().expect("Failed to eval Lua");
        PluginSpec::from_lua(value, &lua).expect("Failed to parse PluginSpec")
    }

    #[test]
    fn plugin_spec_name_from_string() {
        let spec = make_plugin_spec(
            r#"
return {
    name = "my-plugin",
    actions = {
        do_thing = {
            exec = function(args) end,
        }
    }
}
"#,
        );
        assert_eq!(spec.name(), "my-plugin");
    }

    #[test]
    fn plugin_spec_summary_from_string() {
        let spec = make_plugin_spec(
            r#"
return {
    name = "test",
    summary = "A test plugin",
    actions = {
        do_thing = {
            exec = function(args) end,
        }
    }
}
"#,
        );
        assert_eq!(spec.summary(), "A test plugin");
    }

    #[test]
    fn plugin_spec_summary_default_when_absent() {
        let spec = make_plugin_spec(
            r#"
return {
    name = "test",
    actions = {
        do_thing = {
            exec = function(args) end,
        }
    }
}
"#,
        );
        // Summary should default to "Plugin <name> completed."
        assert!(spec.summary().contains("test"));
    }

    #[test]
    fn plugin_spec_get_action_found() {
        let spec = make_plugin_spec(
            r#"
return {
    name = "test",
    actions = {
        my_action = {
            exec = function(args) end,
        }
    }
}
"#,
        );
        assert!(spec.get_action("my_action").is_some());
    }

    #[test]
    fn plugin_spec_get_action_not_found() {
        let spec = make_plugin_spec(
            r#"
return {
    name = "test",
    actions = {
        my_action = {
            exec = function(args) end,
        }
    }
}
"#,
        );
        assert!(spec.get_action("nonexistent").is_none());
    }

    #[test]
    fn plugin_spec_exec_action() {
        let spec = make_plugin_spec(
            r#"
return {
    name = "test",
    actions = {
        greet = {
            exec = function(args) return "hello" end,
        }
    }
}
"#,
        );
        let result = spec.exec_action("greet", Value::Nil);
        assert!(result.is_ok());
    }

    #[test]
    fn plugin_spec_exec_action_not_found_errors() {
        let spec = make_plugin_spec(
            r#"
return {
    name = "test",
    actions = {
        greet = {
            exec = function(args) return "hello" end,
        }
    }
}
"#,
        );
        let result = spec.exec_action("missing", Value::Nil);
        assert!(result.is_err());
    }

    #[test]
    fn plugin_spec_plan_action_with_plan() {
        let spec = make_plugin_spec(
            r#"
return {
    name = "test",
    actions = {
        my_action = {
            plan = function(args) return "planned" end,
            exec = function(args) end,
        }
    }
}
"#,
        );
        let result = spec.plan_action("my_action", Value::Nil);
        assert!(result.is_some());
    }

    #[test]
    fn plugin_spec_plan_action_without_plan_returns_none() {
        let spec = make_plugin_spec(
            r#"
return {
    name = "test",
    actions = {
        my_action = {
            exec = function(args) end,
        }
    }
}
"#,
        );
        let result = spec.plan_action("my_action", Value::Nil);
        assert!(result.is_none());
    }

    #[test]
    fn plugin_spec_display() {
        let spec = make_plugin_spec(
            r#"
return {
    name = "display-test",
    actions = {
        act = { exec = function() end }
    }
}
"#,
        );
        assert_eq!(format!("{spec}"), "plugin.display-test");
    }

    #[test]
    fn plugin_spec_from_non_table_errors() {
        let lua = Lua::new();
        let value = Value::Integer(42);
        let result = PluginSpec::from_lua(value, &lua);
        assert!(result.is_err());
    }

    #[test]
    fn comparable_lua_hash_and_eq() {
        use std::collections::hash_map::DefaultHasher;
        use std::hash::{Hash, Hasher};

        let lua1 = ComparableLua::default();
        let lua2 = ComparableLua::default();

        // They should not be equal (different instances)
        assert_ne!(lua1, lua2);

        // Hashing should not panic
        let mut h = DefaultHasher::new();
        lua1.hash(&mut h);
        let _ = h.finish();
    }

    #[test]
    fn comparable_lua_ord() {
        let lua1 = ComparableLua::default();
        let lua2 = ComparableLua::default();
        let _ = lua1.partial_cmp(&lua2);
        let _ = lua1.cmp(&lua2);
    }

    #[test]
    fn comparable_function_eq_and_hash() {
        use std::collections::hash_map::DefaultHasher;
        use std::hash::{Hash, Hasher};

        let lua = Lua::new();
        let f1 = lua.create_function(|_, ()| Ok(())).unwrap();
        let cf1 = ComparableFunction(f1.clone());
        let cf2 = ComparableFunction(f1);
        // Same function pointer should be equal
        assert_eq!(cf1, cf2);

        let mut h = DefaultHasher::new();
        cf1.hash(&mut h);
        let _ = h.finish();
    }

    #[test]
    fn comparable_function_ord() {
        let lua = Lua::new();
        let f1 = lua.create_function(|_, ()| Ok(())).unwrap();
        let f2 = lua.create_function(|_, ()| Ok(())).unwrap();
        let cf1 = ComparableFunction(f1);
        let cf2 = ComparableFunction(f2);
        let _ = cf1.partial_cmp(&cf2);
        let _ = cf1.cmp(&cf2);
    }

    #[test]
    fn plugin_spec_name_from_function() {
        let spec = make_plugin_spec(
            r#"
return {
    name = function() return "dynamic-name" end,
    actions = {
        act = { exec = function() end }
    }
}
"#,
        );
        assert_eq!(spec.name(), "dynamic-name");
    }

    #[test]
    fn plugin_spec_summary_from_function() {
        let spec = make_plugin_spec(
            r#"
return {
    name = "test",
    summary = function() return "dynamic summary" end,
    actions = {
        act = { exec = function() end }
    }
}
"#,
        );
        assert_eq!(spec.summary(), "dynamic summary");
    }

    #[test]
    fn plugin_spec_with_privileged_action() {
        let spec = make_plugin_spec(
            r#"
return {
    name = "test",
    actions = {
        root_action = {
            is_privileged = true,
            exec = function(args) end,
        }
    }
}
"#,
        );
        let action = spec.get_action("root_action").unwrap();
        assert!(action.is_privileged);
    }
}
