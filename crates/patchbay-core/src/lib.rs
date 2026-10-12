//! Platform-agnostic core for patchbay: configuration, domain rules, tunnel
//! engines, the daemon lifecycle, and platform integration.

pub mod config;
pub mod daemon;
pub mod domains;
pub mod engine;
pub mod interception;
pub mod json;
pub mod options;
pub mod paths;
pub mod platform;
pub mod servers;
