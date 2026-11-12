//! Sunstack host library: the engine facade shared by the CLI and SDK adapters.
//!
//! Domain crates stay free of Node and NAPI; adapters depend on this facade.
pub mod host_services;
pub mod sdk_host;
