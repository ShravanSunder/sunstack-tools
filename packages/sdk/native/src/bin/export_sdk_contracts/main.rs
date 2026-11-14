//! Generate the SDK's TypeScript declarations from the Rust-owned contracts.
//!
//! Outputs, into the directory given as the first argument:
//! - `contracts.ts`: declarations for every SDK input and outcome type;
//! - `wide-integers.json`: where full-width integers sit, so replies decode losslessly;
//! - `ast-grep.schema.json`: the pinned upstream matcher schema the matcher
//!   declarations are generated from;
//! - `contract.json`: the release and contract revision the declarations describe.
mod contract_export;

use std::{error::Error, fs, path::PathBuf};

fn main() -> Result<(), Box<dyn Error>> {
    let directory = PathBuf::from(
        std::env::args_os()
            .nth(1)
            .ok_or("Supply the generated-contracts output directory")?,
    );
    fs::create_dir_all(&directory)?;
    let exported = contract_export::export()?;
    fs::write(directory.join("contracts.ts"), exported.declarations)?;
    fs::write(
        directory.join("wide-integers.json"),
        serde_json::to_string_pretty(&exported.wide_integers)? + "\n",
    )?;
    fs::write(
        directory.join("ast-grep.schema.json"),
        serde_json::to_string_pretty(&sunstack::sdk_host::matcher_transport::matcher_schema())?
            + "\n",
    )?;
    fs::write(
        directory.join("contract.json"),
        serde_json::to_string_pretty(&sunstack::sdk_host::contract_info("declarations"))? + "\n",
    )?;
    Ok(())
}
