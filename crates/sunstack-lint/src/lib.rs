//! Lint repository documents through the native Sunstack engine.
#[path = "configuration_loading/config_contracts.rs"]
mod config_contracts;
pub use config_contracts::*;
#[path = "configuration_loading/config.rs"]
pub mod config;
#[path = "configuration_loading/config_decoding.rs"]
mod config_decoding;
pub use config::*;
#[path = "configuration_loading/limit_settings.rs"]
mod limit_settings;
pub use limit_settings::*;
#[path = "lint_reporting/report_contracts.rs"]
mod report_contracts;
pub use report_contracts::*;
#[path = "configuration_loading/config_validation.rs"]
mod config_validation;
pub use config_validation::{check, list_rules};
#[path = "lint_execution/backend_factory.rs"]
mod backend_factory;
#[path = "lint_execution/lint_run.rs"]
mod lint_run;
#[path = "lint_reporting/outcome.rs"]
pub mod outcome;
pub use lint_run::{run, run_with_source};
#[path = "lint_reporting/formats.rs"]
pub mod formats;
pub use formats::{OutputFormat, render};
#[path = "lint_reporting/sarif_format.rs"]
mod sarif_format;
#[path = "lint_execution/targets.rs"]
pub mod targets;
