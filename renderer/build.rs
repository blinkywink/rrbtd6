// Embeds RustTD6's icon (assets/brand/icon.ico) in the Windows executable.
fn main() {
    println!("cargo:rerun-if-changed=../assets/brand/icon.ico");
    if std::env::var("CARGO_CFG_TARGET_OS").as_deref() == Ok("windows") {
        let mut res = winresource::WindowsResource::new();
        res.set_icon("../assets/brand/icon.ico");
        if let Err(e) = res.compile() {
            println!("cargo:warning=icon not embedded: {e}");
        }
    }
}
