use std::{env, error::Error, fs, path::PathBuf};

fn main() -> Result<(), Box<dyn Error>> {
    println!("cargo:rerun-if-changed=build.rs");
    if env::var("CARGO_CFG_TARGET_VENDOR")? != "apple" {
        return Ok(());
    }

    let package_version = env::var("CARGO_PKG_VERSION")?;
    let plist_path =
        PathBuf::from(env::var_os("OUT_DIR").ok_or("OUT_DIR is unavailable")?).join("Info.plist");
    // codesign reads this identifier when cargo-dist signs without --identifier.
    fs::write(
        &plist_path,
        format!(
            r#"<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
    <key>CFBundleIdentifier</key><string>dev.shravansunder.sunstack</string>
    <key>CFBundleName</key><string>sunstack</string>
    <key>CFBundleShortVersionString</key><string>{package_version}</string>
</dict>
</plist>
"#
        ),
    )?;
    println!(
        "cargo:rustc-link-arg-bin=sunstack=-Wl,-sectcreate,__TEXT,__info_plist,{}",
        plist_path.display()
    );
    Ok(())
}
