#[allow(clippy::module_inception)]
mod plugin;

use std::{
    collections::BTreeMap,
    ops::BitOr,
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
    /// Computation-only libraries: no `io`, `os`, `package` or `debug`, no `dofile` or
    /// `loadfile`, and `load` accepts source text only.
    Restricted,
    /// Every Lua standard library, including `debug`. (`ffi` is LuaJIT-only; this crate
    /// builds Lua 5.4, which has none.)
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

/// Libraries a restricted plugin gets: computation only, nothing that reaches the host.
const RESTRICTED_LIBS: [StdLib; 5] = [
    StdLib::COROUTINE,
    StdLib::TABLE,
    StdLib::STRING,
    StdLib::UTF8,
    StdLib::MATH,
];

fn new_lua(authority: PluginAuthority) -> Result<Lua> {
    match authority {
        PluginAuthority::Full => {
            #[expect(
                unsafe_code,
                reason = "Lua::unsafe_new + StdLib::ALL (debug): plugin code runs with full process authority on a real apply only; the plugin source (possibly a third-party GitHub repo at a mutable tag) is trusted because the operator names it"
            )]
            let lua = unsafe { Lua::unsafe_new() };
            lua.load_std_libs(StdLib::ALL)?;
            Ok(lua)
        }
        PluginAuthority::Restricted => {
            // Folded with `BitOr::bitor` rather than written `a | b | ...`: the flags are
            // disjoint bits, so cargo-mutants' `|` -> `^` mutants would be equivalent and
            // unkillable, failing the required mutation-pr check.
            let libs = RESTRICTED_LIBS.into_iter().fold(StdLib::NONE, BitOr::bitor);
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
        // The env-forwarding branch of the wrapper must refuse bytecode too.
        let loaded: bool = lua
            .load("return load(string.dump(function() end), 'c', 'b', {}) ~= nil")
            .eval()?;
        assert!(
            !loaded,
            "restricted Lua loaded a precompiled chunk with an env"
        );
        Ok(())
    }

    #[test]
    fn restricted_lua_load_still_compiles_source_text() -> Result<()> {
        let lua = new_lua(PluginAuthority::Restricted)?;
        // Reads a global, so it fails if an absent env were forwarded as an explicit nil.
        let value: i64 = lua.load("return load('return math.max(1, 2)')()").eval()?;
        assert_eq!(value, 2);
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

    /// Resets the process-global authority when dropped, so a failing assertion cannot
    /// leave later tests in this process running with full authority.
    struct AuthorityGuard;

    impl Drop for AuthorityGuard {
        fn drop(&mut self) {
            set_plugin_authority(PluginAuthority::Restricted);
        }
    }

    #[test]
    #[serial_test::serial]
    fn plugin_authority_follows_set_plugin_authority() {
        let _guard = AuthorityGuard;
        assert_eq!(plugin_authority(), PluginAuthority::Restricted);
        set_plugin_authority(PluginAuthority::Full);
        assert_eq!(plugin_authority(), PluginAuthority::Full);
        set_plugin_authority(PluginAuthority::Restricted);
        assert_eq!(plugin_authority(), PluginAuthority::Restricted);
    }

    #[test]
    #[serial_test::serial]
    fn plugin_cache_is_keyed_by_authority() -> anyhow::Result<()> {
        let _guard = AuthorityGuard;
        let dir = tempfile::tempdir()?;
        let lua_file = dir.path().join("plugin.lua");
        std::fs::write(
            &lua_file,
            r#"return { summary = io and "full" or "restricted", actions = {} }"#,
        )?;
        let plugin: Plugin = serde_json::from_value(serde_json::json!({
            "dir": lua_file,
            "actions": {}
        }))?;

        use crate::actions::Action;
        set_plugin_authority(PluginAuthority::Restricted);
        assert_eq!(plugin.summarize(), "restricted");
        set_plugin_authority(PluginAuthority::Full);
        assert_eq!(plugin.summarize(), "full");
        set_plugin_authority(PluginAuthority::Restricted);
        assert_eq!(plugin.summarize(), "restricted");
        Ok(())
    }
}
