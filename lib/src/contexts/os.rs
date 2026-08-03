use crate::contexts::{Context, ContextProvider};
use anyhow::Result;
use gethostname::gethostname;
use os_info;

#[derive(Debug)]
pub struct OSContextProvider {}

impl ContextProvider for OSContextProvider {
    fn get_prefix(&self) -> String {
        String::from("os")
    }

    fn get_contexts(&self) -> Result<Vec<super::Context>> {
        let osinfo = os_info::get();

        Ok(vec![
            Context::KeyValueContext(String::from("hostname"), gethostname().into()),
            Context::KeyValueContext(String::from("family"), std::env::consts::FAMILY.into()),
            Context::KeyValueContext(String::from("name"), std::env::consts::OS.into()),
            Context::KeyValueContext(String::from("arch"), std::env::consts::ARCH.into()),
            Context::KeyValueContext(
                String::from("distribution"),
                format!("{}", osinfo.os_type()).into(),
            ),
            Context::KeyValueContext(
                String::from("codename"),
                String::from(osinfo.codename().unwrap_or("unknown")).into(),
            ),
            Context::KeyValueContext(
                String::from("bitness"),
                format!("{}", osinfo.bitness()).into(),
            ),
            Context::KeyValueContext(
                String::from("version"),
                format!("{}", osinfo.version()).into(),
            ),
            Context::KeyValueContext(
                String::from("edition"),
                String::from(osinfo.edition().unwrap_or("unknown")).into(),
            ),
        ])
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn it_can_prefix() {
        let oscontext = OSContextProvider {};
        let prefix = oscontext.get_prefix();
        assert_eq!(String::from("os"), prefix);
    }

    #[test]
    fn arch_field_is_present() {
        let oscontext = OSContextProvider {};
        let contexts = oscontext.get_contexts().unwrap();
        let has_arch = contexts
            .iter()
            .any(|c| matches!(c, Context::KeyValueContext(k, _) if k == "arch"));
        assert!(has_arch, "expected 'arch' key in OS contexts");
    }

    #[test]
    fn arch_field_matches_consts() {
        let oscontext = OSContextProvider {};
        let contexts = oscontext.get_contexts().unwrap();
        let arch_val = contexts.iter().find_map(|c| match c {
            Context::KeyValueContext(k, v) if k == "arch" => Some(v.to_string()),
            _ => None,
        });
        assert_eq!(arch_val.unwrap(), std::env::consts::ARCH);
    }

    #[test]
    fn arch_field_is_non_empty() {
        let oscontext = OSContextProvider {};
        let contexts = oscontext.get_contexts().unwrap();
        let arch_val = contexts.iter().find_map(|c| match c {
            Context::KeyValueContext(k, v) if k == "arch" => Some(v.to_string()),
            _ => None,
        });
        assert!(!arch_val.unwrap().is_empty());
    }

    #[test]
    #[cfg(target_os = "macos")]
    fn it_can_macos() {
        let oscontext = OSContextProvider {};
        let keyvaluepairs = oscontext.get_contexts().unwrap();

        keyvaluepairs.iter().for_each(|context| match context {
            Context::KeyValueContext(k, v) => match k.as_ref() {
                "family" => assert_eq!(v.to_string(), String::from("unix")),
                "name" => assert_eq!(v.to_string(), String::from("macos")),
                "arch" => assert!(!v.to_string().is_empty()),
                _ => (),
            },
        })
    }

    #[test]
    #[cfg(all(target_os = "macos", target_arch = "aarch64"))]
    fn it_can_macos_apple_silicon() {
        let oscontext = OSContextProvider {};
        let keyvaluepairs = oscontext.get_contexts().unwrap();
        let arch_val = keyvaluepairs.iter().find_map(|c| match c {
            Context::KeyValueContext(k, v) if k == "arch" => Some(v.to_string()),
            _ => None,
        });
        assert_eq!(arch_val.unwrap(), "aarch64");
    }

    #[test]
    #[cfg(target_os = "linux")]
    fn it_can_linux() {
        let oscontext = OSContextProvider {};
        let keyvaluepairs = oscontext.get_contexts().unwrap();

        keyvaluepairs.iter().for_each(|context| match context {
            Context::KeyValueContext(k, v) => match k.as_ref() {
                "family" => assert_eq!(v.to_string(), String::from("unix")),
                "name" => assert_eq!(v.to_string(), String::from("linux")),
                "arch" => assert!(!v.to_string().is_empty()),
                _ => (),
            },
        })
    }
}
