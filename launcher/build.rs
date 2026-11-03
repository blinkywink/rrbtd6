//! Links our app icon (renderer/assets/icon/icon.res, made by tools/assets/make_icon.py) into the
//! launcher EXE on Windows MSVC builds.
fn main() {
    let res = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../renderer/assets/icon/icon.res");
    println!("cargo:rerun-if-changed={}", res.display());
    if std::env::var("CARGO_CFG_TARGET_OS").as_deref() == Ok("windows") && std::env::var("CARGO_CFG_TARGET_ENV").as_deref() == Ok("msvc") {
        println!("cargo:rustc-link-arg-bins={}", res.display());
    }
}
