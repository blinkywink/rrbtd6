//! Links our app icon (assets/icon/icon.res, made by tools/assets/make_icon.py) into the EXE on
//! Windows MSVC builds; elsewhere the window icon alone (main.rs) carries it.
fn main() {
    // The game's version (window title, crash log): VERSION at the repository root.
    let version_file = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../VERSION");
    println!("cargo:rerun-if-changed={}", version_file.display());
    let version = std::fs::read_to_string(&version_file).map(|v| v.trim().to_string()).unwrap_or_else(|_| "dev".into());
    println!("cargo:rustc-env=RRBTD6_VERSION={version}");
    let res = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("assets/icon/icon.res");
    println!("cargo:rerun-if-changed={}", res.display());
    if std::env::var("CARGO_CFG_TARGET_OS").as_deref() == Ok("windows") && std::env::var("CARGO_CFG_TARGET_ENV").as_deref() == Ok("msvc") {
        println!("cargo:rustc-link-arg-bins={}", res.display());
        // Switchable-graphics laptops: the drivers read these exports (src/platform.rs).
        println!("cargo:rustc-link-arg-bins=/EXPORT:NvOptimusEnablement");
        println!("cargo:rustc-link-arg-bins=/EXPORT:AmdPowerXpressRequestHighPerformance");
    }
}
