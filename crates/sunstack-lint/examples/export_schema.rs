use std::io::{self, Write};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut stdout = io::stdout().lock();
    serde_json::to_writer_pretty(&mut stdout, &sunstack_lint::config_schema())?;
    stdout.write_all(b"\n")?;
    Ok(())
}
