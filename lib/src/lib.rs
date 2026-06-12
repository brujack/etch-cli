// Pre-existing patterns — redundant .into_iter() calls that
// clippy flags on Linux but not macOS; not worth changing inherited code.
#![allow(clippy::useless_conversion)]

pub mod actions;
pub mod atoms;
pub mod config;
pub mod contexts;
pub mod doctor;
pub mod manifests;
pub mod state;
pub mod steps;
pub mod tera_functions;
mod utilities;
pub mod values;

#[cfg(test)]
pub mod test_helpers;
