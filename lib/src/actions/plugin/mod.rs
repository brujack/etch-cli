#[allow(clippy::module_inception)]
mod plugin;

use std::{
    collections::BTreeMap,
    sync::{
        OnceLock,
        atomic::{AtomicBool, Ordering},
    },
};

use parking_lot::Mutex;
use tealr::mlu::mlua::{Lua, LuaOptions, Result, StdLib, Value};

use crate::{
    atoms::plugin::{PluginSpec, setup_globals},
    contexts::Contexts,
};
pub use plugin::Plugin;
use plugin::{RepoOrDir, Source};

/// How much of the host a plugin's Lua code can reach.
///
/// The default is [`PluginAuthority::Restricted`]. Read-only commands such as
/// `etch apply --dry-run` and `etch status` evaluate plugins to plan and summarize them,
/// and must not hand plugin code the operator's filesystem, processes or environment
/// (audit 2026-11-16 F15). Only a real apply grants [`PluginAuthority::Full`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum PluginAuthority {
    /// Computation-only libraries: no `io`, `os`, `package`, `debug` or `ffi`, and no
    /// `dofile` or `loadfile`.
    Restricted,
    /// Every Lua standard library, including `debug` and `ffi`.
    Full,
}

static FULL_AUTHORITY: AtomicBool = AtomicBool::new(false);

/// Sets the authority for every plugin evaluated from now on in this process.
pub fn set_plugin_authority(authority: PluginAuthority) {
    FULL_AUTHORITY.store(authority == PluginAuthority::Full, Ordering::SeqCst);
}

/// The authority plugins are currently evaluated with.
pub fn plugin_authority() -> PluginAuthority {
    if FULL_AUTHORITY.load(Ordering::SeqCst) {
        PluginAuthority::Full
    } else {
        PluginAuthority::Restricted
    }
}

fn new_lua(authority: PluginAuthority) -> Result<Lua> {
    match authority {
        PluginAuthority::Full => {
            #[expect(
                unsafe_code,
                reason = "Lua::unsafe_new + StdLib::ALL (debug/ffi): plugin code runs with full process authority on a real apply only; the plugin source (possibly a third-party GitHub repo at a mutable tag) is trusted because the operator names it"
            )]
            let lua = unsafe { Lua::unsafe_new() };
            lua.load_std_libs(StdLib::ALL)?;
            Ok(lua)
        }
        PluginAuthority::Restricted => {
            let libs =
                StdLib::COROUTINE | StdLib::TABLE | StdLib::STRING | StdLib::UTF8 | StdLib::MATH;
            let lua = Lua::new_with(libs, LuaOptions::default())?;
            // The base library is always loaded, and these two read files from disk.
            let globals = lua.globals();
            globals.set("dofile", Value::Nil)?;
            globals.set("loadfile", Value::Nil)?;
            // Precompiled chunks can corrupt interpreter memory and escape the restriction,
            // so `load` accepts source text only. `env` is forwarded only when given,
            // because Lua treats an explicit nil env differently from an absent one.
            lua.load(
                r##"
local raw_load = load
load = function(chunk, name, _, ...)
    if select("#", ...) > 0 then
        return raw_load(chunk, name, "t", (...))
    end
    return raw_load(chunk, name, "t")
end
"##,
            )
            .exec()?;
            Ok(lua)
        }
    }
}

fn get_plugin(source: &RepoOrDir, contexts: Option<Contexts>) -> Result<PluginSpec> {
    let authority = plugin_authority();
    // Keyed by authority as well, so a spec evaluated restricted is never reused by a
    // full-authority apply in the same process, or the reverse.
    let key = (source.to_owned(), authority);
    let mut plugins = PLUGINS.get_or_init(|| Mutex::new(BTreeMap::new())).lock();

    if let Some(spec) = plugins.get(&key).cloned() {
        return Ok(spec);
    }

    let lua = new_lua(authority)?;

    if let Some(contexts) = contexts {
        setup_globals(&lua, contexts)?;
    }

    let plugin_spec: PluginSpec = lua.load(source.source()?).eval()?;

    plugins.insert(key, plugin_spec.clone());

    Ok(plugin_spec)
}

type PluginCache = BTreeMap<(RepoOrDir, PluginAuthority), PluginSpec>;

static PLUGINS: OnceLock<Mutex<PluginCache>> = OnceLock::new();

#[cfg(test)]
mod tests {
    use super::*;

    fn global_is_nil(lua: &Lua, name: &str) -> bool {
        matches!(lua.globals().get::<Value>(name), Ok(Value::Nil))
    }

    #[test]
    fn restricted_lua_has_no_host_reaching_globals() -> Result<()> {
        let lua = new_lua(PluginAuthority::Restricted)?;
        for name in [
            "io", "os", "package", "require", "debug", "ffi", "dofile", "loadfile",
        ] {
            assert!(global_is_nil(&lua, name), "restricted Lua exposes `{name}`");
        }
        Ok(())
    }

    #[test]
    fn restricted_lua_keeps_computation_libraries() -> Result<()> {
        let lua = new_lua(PluginAuthority::Restricted)?;
        for name in ["string", "table", "math", "utf8", "coroutine", "print"] {
            assert!(!global_is_nil(&lua, name), "restricted Lua lacks `{name}`");
        }
        Ok(())
    }

    #[test]
    fn restricted_lua_refuses_binary_chunks() -> Result<()> {
        let lua = new_lua(PluginAuthority::Restricted)?;
        let loaded: bool = lua
            .load("return load(string.dump(function() end)) ~= nil")
            .eval()?;
        assert!(!loaded, "restricted Lua loaded a precompiled chunk");
        Ok(())
    }

    #[test]
    fn restricted_lua_load_still_compiles_source_text() -> Result<()> {
        let lua = new_lua(PluginAuthority::Restricted)?;
        let value: i64 = lua.load("return load('return 1 + 2')()").eval()?;
        assert_eq!(value, 3);
        // An explicit env is honoured: the chunk sees `x` from the table it was given.
        let value: i64 = lua
            .load("return load('return x', 'c', 'b', { x = 7 })()")
            .eval()?;
        assert_eq!(value, 7);
        Ok(())
    }

    // Positive control for the restricted tests: the same probe sees every library.
    #[test]
    fn full_lua_exposes_host_reaching_globals() -> Result<()> {
        let lua = new_lua(PluginAuthority::Full)?;
        for name in ["io", "os", "package", "debug", "dofile", "loadfile"] {
            assert!(!global_is_nil(&lua, name), "full Lua lacks `{name}`");
        }
        Ok(())
    }
}
