//! Links our app icon (assets/icon/icon.res, made by tools/assets/make_icon.py) into the EXE on
//! Windows MSVC builds; elsewhere the window icon alone (main.rs) carries it.
fn main() {
    let res = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("assets/icon/icon.res");
    println!("cargo:rerun-if-changed={}", res.display());
    if std::env::var("CARGO_CFG_TARGET_OS").as_deref() == Ok("windows") && std::env::var("CARGO_CFG_TARGET_ENV").as_deref() == Ok("msvc") {
        println!("cargo:rustc-link-arg-bins={}", res.display());
    }
}
