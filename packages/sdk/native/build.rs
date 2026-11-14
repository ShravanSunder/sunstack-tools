fn main() {
    napi_build::setup();
    // The contract handshake reports the exact compiled target.
    if let Ok(target) = std::env::var("TARGET") {
        println!("cargo:rustc-env=SUNSTACK_SDK_TARGET={target}");
    }
}
