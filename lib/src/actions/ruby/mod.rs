mod install;
pub(crate) use install::RubyInstall;
// VersionManager is part of the public API (type of RubyInstall::version_manager field)
#[allow(unused_imports)]
pub use install::VersionManager;
