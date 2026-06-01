mod autoremove;
mod install;
mod providers;
mod repository;

pub(crate) use autoremove::PackageAutoremove;
pub(crate) use install::PackageInstall;
use providers::PackageProviders;
pub(crate) use repository::PackageRepository;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use tracing::debug;

#[derive(JsonSchema, Clone, Debug, Default, Serialize, Deserialize)]
#[serde(rename = "package.install")]
pub struct Package {
    name: Option<String>,

    #[serde(default)]
    list: Vec<String>,

    #[serde(default)]
    provider: PackageProviders,

    #[serde(default)]
    repository: Option<String>,

    #[serde(default)]
    key: Option<String>,

    #[serde(default)]
    extra_args: Vec<String>,

    #[serde(default)]
    variants: HashMap<os_info::Type, PackageVariant>,

    #[serde(default)]
    file: bool,

    #[serde(default)]
    cask: bool,
}

#[derive(JsonSchema, Clone, Debug, Default, Serialize, Deserialize)]
pub struct PackageVariant {
    name: Option<String>,

    #[serde(default)]
    list: Vec<String>,

    #[serde(default)]
    provider: PackageProviders,

    #[serde(default)]
    extra_args: Vec<String>,

    #[serde(default)]
    file: bool,

    #[serde(default)]
    cask: bool,
}

impl PackageVariant {
    fn packages(&self) -> Vec<String> {
        self.name
            .as_ref()
            .map(|s| vec![s.clone()])
            .unwrap_or_else(|| self.list.clone())
    }
}

impl From<&Package> for PackageVariant {
    fn from(package: &Package) -> Self {
        let os = os_info::get();

        // Check for variant configuration for this OS
        let variant = package.variants.get(&os.os_type());

        // No variant overlays
        if variant.is_none() {
            return PackageVariant {
                name: package.name.clone(),
                list: package.list.clone(),
                provider: package.provider.clone(),
                extra_args: package.extra_args.clone(),
                file: package.file,
                cask: package.cask,
            };
        };

        // .unwrap() is safe here because we checked for None above
        let variant = variant.unwrap();

        debug!(message = "Built Variant", variant = ?variant);

        let mut package = PackageVariant {
            name: package.name.clone(),
            list: package.list.clone(),
            provider: variant.provider.clone(),
            extra_args: variant.extra_args.clone(),
            file: package.file,
            cask: variant.cask,
        };

        if variant.name.is_some() {
            package.name = variant.name.clone();
        }

        if !variant.list.is_empty() {
            package.list = variant.list.clone();
        }

        // I've been torn about this, but here's my logic.
        // Variants, when being used, shouldn't use the provider
        // of the main definition; as we're not the core OS.
        // Even if the omission of a provider for a variant gets us
        // the default, that's most likely still expected behaviour.
        // Right?
        package.provider = variant.provider.clone();

        package
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn package_variant_from_package_with_name() {
        let pkg = Package {
            name: Some("curl".to_string()),
            ..Default::default()
        };
        let variant: PackageVariant = (&pkg).into();
        assert_eq!(variant.packages(), vec!["curl"]);
    }

    #[test]
    fn package_variant_from_package_with_list() {
        let pkg = Package {
            list: vec!["git".to_string(), "vim".to_string()],
            ..Default::default()
        };
        let variant: PackageVariant = (&pkg).into();
        assert_eq!(variant.packages(), vec!["git", "vim"]);
    }

    #[test]
    fn package_variant_packages_empty_when_no_name_or_list() {
        let pkg = Package::default();
        let variant: PackageVariant = (&pkg).into();
        assert!(variant.packages().is_empty());
    }

    #[test]
    fn package_variant_from_package_with_matching_os_variant() {
        use std::collections::HashMap;

        let os = os_info::get();
        let mut variants = HashMap::new();
        variants.insert(
            os.os_type(),
            PackageVariant {
                name: Some("variant-pkg".to_string()),
                list: vec!["extra-pkg".to_string()],
                ..Default::default()
            },
        );

        let pkg = Package {
            name: Some("base-pkg".to_string()),
            variants,
            ..Default::default()
        };

        let variant: PackageVariant = (&pkg).into();
        // variant.name overrides base name; variant.list overrides base list
        assert_eq!(variant.packages(), vec!["variant-pkg"]);
    }

    #[test]
    fn package_variant_from_package_with_cask() {
        let pkg = Package {
            name: Some(String::from("alfred")),
            cask: true,
            ..Default::default()
        };
        let variant: PackageVariant = (&pkg).into();
        assert!(variant.cask);
        assert_eq!(variant.packages(), vec!["alfred"]);
    }

    #[test]
    fn package_variant_from_package_cask_overridden_by_variant() {
        let os = os_info::get();
        let mut variants = std::collections::HashMap::new();
        variants.insert(
            os.os_type(),
            PackageVariant {
                name: Some(String::from("variant-cask")),
                cask: true,
                ..Default::default()
            },
        );
        let pkg = Package {
            name: Some(String::from("base-formula")),
            cask: false,
            variants,
            ..Default::default()
        };
        let variant: PackageVariant = (&pkg).into();
        assert!(
            variant.cask,
            "variant cask:true should override base cask:false"
        );
        assert_eq!(variant.packages(), vec!["variant-cask"]);
    }
}
