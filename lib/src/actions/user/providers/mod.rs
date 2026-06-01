use crate::steps::Step;
mod none;
use self::none::NoneUserProvider;
use super::{add_group::UserAddGroup, UserVariant};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
mod linux;
use self::linux::LinuxUserProvider;
mod macos;
use self::macos::MacOSUserProvider;

use crate::contexts::Contexts;

#[derive(JsonSchema, Clone, Debug, Serialize, Deserialize)]
pub enum UserProviders {
    #[serde(alias = "none")]
    None,

    #[serde(alias = "linux")]
    Linux,

    #[serde(alias = "macos")]
    MacOs,
}

impl UserProviders {
    pub fn get_provider(self) -> Box<dyn UserProvider> {
        match self {
            UserProviders::None => Box::new(NoneUserProvider {}),
            UserProviders::Linux => Box::new(LinuxUserProvider {}),
            UserProviders::MacOs => Box::new(MacOSUserProvider {}),
        }
    }
}

impl Default for UserProviders {
    #[cfg(target_os = "linux")]
    fn default() -> Self {
        UserProviders::Linux
    }

    #[cfg(not(target_os = "linux"))]
    fn default() -> Self {
        let info = os_info::get();

        match info.os_type() {
            os_info::Type::Macos => UserProviders::MacOs,
            _ => UserProviders::None,
        }
    }
}

pub trait UserProvider {
    fn add_user(&self, user: &UserVariant, contexts: &Contexts) -> anyhow::Result<Vec<Step>>;
    fn add_to_group(&self, user: &UserAddGroup, contexts: &Contexts) -> anyhow::Result<Vec<Step>>;
}

/// Returns true when `username` is already a member of `group`.
/// Uses `id -nG <username>` — works on both Linux and macOS.
/// Returns false on any error (user not found, id not in PATH) so callers
/// fail-safe by generating the membership step rather than skipping it.
pub(super) fn user_in_group(username: &str, group: &str) -> bool {
    std::process::Command::new("id")
        .args(["-nG", username])
        .output()
        .map(|o| {
            o.status.success()
                && String::from_utf8_lossy(&o.stdout)
                    .split_whitespace()
                    .any(|g| g == group)
        })
        .unwrap_or(false)
}
