//! Lint repository documents through the native Sunstack engine.
pub mod configuration_loading;
pub mod lint_execution;
pub mod lint_reporting;

pub use configuration_loading::config::*;
pub use configuration_loading::config_contracts::*;
pub use configuration_loading::config_validation::{check, list_rules};
pub use configuration_loading::limit_settings::*;
pub use lint_execution::lint_run::{run, run_with_source};
pub use lint_reporting::formats::{OutputFormat, render};
pub use lint_reporting::report_contracts::*;
