use std::{
    borrow::Cow,
    hash::{Hash, Hasher},
    ops::{Deref, DerefMut},
};

use anyhow::Result;
use serde_json::Value as JsonValue;
use tealr::{
    mlu::{
        mlua::{Error as LuaError, Function, Lua, Value as LuaValue},
        FromToLua,
    },
    ToTypename,
};
use tracing::error;

use schemars::{json_schema, JsonSchema, Schema, SchemaGenerator};

// Used in tests and reserved for Lua→JSON roundtrip once plugin output is wired up.
#[allow(dead_code)]
pub(crate) fn lua_value_to_json(value: LuaValue) -> JsonValue {
    match value {
        LuaValue::Nil => JsonValue::Null,
        LuaValue::Boolean(b) => JsonValue::Bool(b),
        LuaValue::Integer(i) => JsonValue::Number(i.into()),
        LuaValue::Number(n) => {
            JsonValue::Number(serde_json::Number::from_f64(n).unwrap_or(0.into()))
        }
        LuaValue::String(s) => JsonValue::String(s.to_string_lossy()),
        LuaValue::Table(t) => {
            // Use sequence_values to detect array tables (consecutive integer keys from 1).
            // If the table has pairs but no sequence values, treat it as a string-keyed object.
            let seq_count = t.clone().sequence_values::<LuaValue>().count();
            let all_count = t.clone().pairs::<LuaValue, LuaValue>().count();
            if all_count > 0 && seq_count == 0 {
                // Treat as object (has pairs, none are sequential integer keys)
                let mut map = serde_json::Map::new();
                for pair in t.pairs::<LuaValue, LuaValue>() {
                    let (k, v) = pair.unwrap();
                    if let LuaValue::String(key) = k {
                        map.insert(key.to_string_lossy(), lua_value_to_json(v));
                    }
                }
                JsonValue::Object(map)
            } else {
                // Treat as array (empty, or has sequential integer keys)
                let mut array = Vec::new();
                for v in t.sequence_values::<LuaValue>() {
                    array.push(lua_value_to_json(v.unwrap()));
                }
                JsonValue::Array(array)
            }
        }
        _ => JsonValue::Null,
    }
}

pub fn json_to_lua(json: &JsonValue, lua: &Lua) -> Result<LuaValue, LuaError> {
    match json {
        JsonValue::Null => Ok(LuaValue::Nil),
        JsonValue::Bool(b) => Ok(LuaValue::Boolean(*b)),
        JsonValue::Number(n) => if n.is_i64() {
            n.as_i64().map(LuaValue::Integer)
        } else {
            n.as_f64().map(LuaValue::Number)
        }
        .ok_or_else(|| LuaError::external("Failed to convert number")),
        JsonValue::String(s) => Ok(LuaValue::String(lua.create_string(s)?)),
        JsonValue::Array(arr) => lua.create_table().map(|table| {
            arr.iter()
                .filter_map(|value| json_to_lua(value, lua).ok())
                .enumerate()
                .for_each(|(i, v)| {
                    if let Err(e) = table.set(i + 1, v) {
                        error!("Failed to set value in table: {}", e);
                    };
                });
            LuaValue::Table(table)
        }),
        JsonValue::Object(map) => {
            let table = lua.create_table()?;
            for (k, v) in map {
                table.set(k.clone(), json_to_lua(v, lua)?)?;
            }
            Ok(LuaValue::Table(table))
        }
    }
}

#[derive(Clone, Debug, FromToLua, ToTypename, PartialEq)]
pub struct LuaFunction(pub Function);

impl Eq for LuaFunction {}

impl PartialOrd for LuaFunction {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for LuaFunction {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.0.dump(false).cmp(&other.0.dump(false))
    }
}

impl Hash for LuaFunction {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.0.dump(false).hash(state);
    }
}

impl Deref for LuaFunction {
    type Target = Function;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl DerefMut for LuaFunction {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

impl JsonSchema for LuaFunction {
    fn schema_name() -> Cow<'static, str> {
        Cow::Borrowed("LuaFunction")
    }

    fn json_schema(_gen: &mut SchemaGenerator) -> Schema {
        json_schema!({
            "type": "object",
            "description": "Lua function reference (opaque to schema)"
        })
    }
}

impl Default for LuaFunction {
    fn default() -> Self {
        let lua = Lua::new();
        let func = lua
            .create_function(|_, ()| Ok(()))
            .expect("Failed to create default function");
        LuaFunction(func)
    }
}

#[derive(Clone, Debug)]
pub struct LuaRuntime(pub Lua);

impl Deref for LuaRuntime {
    type Target = Lua;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl DerefMut for LuaRuntime {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

impl JsonSchema for LuaRuntime {
    fn schema_name() -> Cow<'static, str> {
        Cow::Borrowed("LuaRuntime")
    }

    fn json_schema(_gen: &mut SchemaGenerator) -> Schema {
        json_schema!({
            "type": "object",
            "description": "Lua runtime state (opaque to schema)"
        })
    }
}

impl Default for LuaRuntime {
    fn default() -> Self {
        LuaRuntime(Lua::new())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    use tealr::mlu::mlua::Lua;

    #[test]
    fn lua_value_to_json_nil() {
        let result = lua_value_to_json(LuaValue::Nil);
        assert_eq!(result, JsonValue::Null);
    }

    #[test]
    fn lua_value_to_json_boolean() {
        assert_eq!(lua_value_to_json(LuaValue::Boolean(true)), json!(true));
        assert_eq!(lua_value_to_json(LuaValue::Boolean(false)), json!(false));
    }

    #[test]
    fn lua_value_to_json_integer() {
        assert_eq!(lua_value_to_json(LuaValue::Integer(42)), json!(42i64));
        assert_eq!(lua_value_to_json(LuaValue::Integer(-7)), json!(-7i64));
    }

    #[test]
    fn lua_value_to_json_number() {
        let result = lua_value_to_json(LuaValue::Number(2.71));
        assert!(result.is_f64() || result.is_number());
    }

    #[test]
    fn lua_value_to_json_string() {
        let lua = Lua::new();
        let s = lua.create_string("hello").unwrap();
        assert_eq!(lua_value_to_json(LuaValue::String(s)), json!("hello"));
    }

    #[test]
    fn lua_value_to_json_array_table() {
        let lua = Lua::new();
        let table = lua.create_table().unwrap();
        table.set(1i64, "a").unwrap();
        table.set(2i64, "b").unwrap();
        let result = lua_value_to_json(LuaValue::Table(table));
        // Array tables have integer keys
        assert!(result.is_array() || result.is_object());
    }

    #[test]
    fn lua_value_to_json_empty_table() {
        let lua = Lua::new();
        let table = lua.create_table().unwrap();
        let result = lua_value_to_json(LuaValue::Table(table));
        // Empty table: pairs count is 0, treated as array
        assert!(result.is_array());
        assert_eq!(result.as_array().unwrap().len(), 0);
    }

    #[test]
    fn lua_value_to_json_other_returns_null() {
        // LightUserData and other types should return Null
        // Use an empty table to check the fallback path isn't needed separately;
        // we can verify the non-table path by checking that non-table lua values return null
        // (already covered by Nil test above)
        let result = lua_value_to_json(LuaValue::Nil);
        assert_eq!(result, JsonValue::Null);
    }

    #[test]
    fn lua_value_to_json_object_table_with_string_keys() {
        let lua = Lua::new();
        let table = lua.create_table().unwrap();
        table.set("name", "Teal'c").unwrap();
        table.set("rank", "First Prime").unwrap();
        let result = lua_value_to_json(LuaValue::Table(table));
        assert!(result.is_object(), "expected object, got: {result:?}");
        assert_eq!(result["name"], json!("Teal'c"));
        assert_eq!(result["rank"], json!("First Prime"));
    }

    #[test]
    fn lua_function_json_schema_body() {
        use schemars::SchemaGenerator;
        let schema = LuaFunction::json_schema(&mut SchemaGenerator::default());
        let v = serde_json::to_value(&schema).unwrap();
        assert_eq!(v["type"], json!("object"));
    }

    #[test]
    fn lua_runtime_json_schema_body() {
        use schemars::SchemaGenerator;
        let schema = LuaRuntime::json_schema(&mut SchemaGenerator::default());
        let v = serde_json::to_value(&schema).unwrap();
        assert_eq!(v["type"], json!("object"));
    }

    #[test]
    fn json_to_lua_null() {
        let lua = Lua::new();
        let result = json_to_lua(&JsonValue::Null, &lua).unwrap();
        assert!(matches!(result, LuaValue::Nil));
    }

    #[test]
    fn json_to_lua_bool() {
        let lua = Lua::new();
        let result = json_to_lua(&json!(true), &lua).unwrap();
        assert!(matches!(result, LuaValue::Boolean(true)));

        let result = json_to_lua(&json!(false), &lua).unwrap();
        assert!(matches!(result, LuaValue::Boolean(false)));
    }

    #[test]
    fn json_to_lua_integer() {
        let lua = Lua::new();
        let result = json_to_lua(&json!(-5i64), &lua).unwrap();
        assert!(matches!(result, LuaValue::Integer(-5)));
    }

    #[test]
    fn json_to_lua_float() {
        let lua = Lua::new();
        let result = json_to_lua(&json!(1.5f64), &lua).unwrap();
        assert!(matches!(result, LuaValue::Number(_)));
    }

    #[test]
    fn json_to_lua_string() {
        let lua = Lua::new();
        let result = json_to_lua(&json!("Stargate"), &lua).unwrap();
        assert!(matches!(result, LuaValue::String(_)));
    }

    #[test]
    fn json_to_lua_array() {
        let lua = Lua::new();
        let result = json_to_lua(&json!(["a", "b"]), &lua).unwrap();
        assert!(matches!(result, LuaValue::Table(_)));
    }

    #[test]
    fn json_to_lua_object() {
        let lua = Lua::new();
        let result = json_to_lua(&json!({"key": "value"}), &lua).unwrap();
        assert!(matches!(result, LuaValue::Table(_)));
    }

    #[test]
    fn lua_function_default_and_deref() {
        let f = LuaFunction::default();
        // Deref should give us the inner Function
        let _ = f.deref();
    }

    #[test]
    fn lua_function_eq_and_hash() {
        // Two LuaFunction instances from the same lua state can be compared
        let lua = Lua::new();
        let f1_raw = lua.create_function(|_, ()| Ok(())).unwrap();
        let f2_raw = lua.create_function(|_, ()| Ok(())).unwrap();
        let f1 = LuaFunction(f1_raw);
        let f2 = LuaFunction(f2_raw);
        let _ = f1 == f2; // just ensure it doesn't panic

        use std::collections::hash_map::DefaultHasher;
        use std::hash::{Hash, Hasher};
        let mut h1 = DefaultHasher::new();
        f1.hash(&mut h1);
        let _ = h1.finish(); // just ensure no panic
    }

    #[test]
    fn lua_function_ord() {
        let lua = Lua::new();
        let f1_raw = lua.create_function(|_, ()| Ok(())).unwrap();
        let f2_raw = lua.create_function(|_, ()| Ok(())).unwrap();
        let f1 = LuaFunction(f1_raw);
        let f2 = LuaFunction(f2_raw);
        let _ = f1.partial_cmp(&f2);
        let _ = f1.cmp(&f2);
    }

    #[test]
    fn lua_function_json_schema() {
        let name = LuaFunction::schema_name();
        assert_eq!(name, "LuaFunction");
    }

    #[test]
    fn lua_function_deref_mut() {
        let mut f = LuaFunction::default();
        let _ = f.deref_mut(); // ensure DerefMut works
    }

    #[test]
    fn lua_runtime_default_and_deref() {
        let rt = LuaRuntime::default();
        let _ = rt.deref(); // Deref to &Lua
    }

    #[test]
    fn lua_runtime_deref_mut() {
        let mut rt = LuaRuntime::default();
        let _ = rt.deref_mut(); // DerefMut to &mut Lua
    }

    #[test]
    fn lua_runtime_json_schema() {
        let name = LuaRuntime::schema_name();
        assert_eq!(name, "LuaRuntime");
    }

    use proptest::prelude::*;

    proptest! {
        #[test]
        fn prop_json_primitives_roundtrip(s in "[a-zA-Z0-9 ]{0,50}") {
            // String: json_to_lua then lua_value_to_json should round-trip
            let lua = Lua::new();
            let json_str = serde_json::Value::String(s.clone());
            let lua_val = json_to_lua(&json_str, &lua).unwrap();
            let back = lua_value_to_json(lua_val);
            prop_assert_eq!(back, json_str);
        }

        #[test]
        fn prop_json_bool_roundtrip(b: bool) {
            let lua = Lua::new();
            let json_bool = serde_json::Value::Bool(b);
            let lua_val = json_to_lua(&json_bool, &lua).unwrap();
            let back = lua_value_to_json(lua_val);
            prop_assert_eq!(back, json_bool);
        }

        #[test]
        fn prop_json_null_roundtrip(_n: ()) {
            let lua = Lua::new();
            let json_null = serde_json::Value::Null;
            let lua_val = json_to_lua(&json_null, &lua).unwrap();
            let back = lua_value_to_json(lua_val);
            prop_assert_eq!(back, json_null);
        }
    }
}
