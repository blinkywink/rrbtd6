//! Asset compatibility is decided before replacing a working executable.
use serde_json::Value;
use std::path::Path;
pub fn needs_assets(release:&Value,assets:&Value)->bool{
    release["assets_revision"].as_str().is_some_and(|required|assets["revision"].as_str()!=Some(required))
}
pub fn extraction_notice(game:&Path,build:u64)->bool{
    let text=format!("RustTD6 build {build} needs newly extracted BTD6 assets.\n\nYour current version is still playable. Co-op requires the same client version for everyone.\n\nRun the RustTD6 setup mod inside BTD6 again to extract the new assets and finish updating. The current game and assets stay in place until extraction succeeds.\n\nOpen BTD6 now? Choose No to keep playing the installed RustTD6 version.");
    #[cfg(windows)]
    let open={
        #[link(name="user32")]
        unsafe extern "system"{fn MessageBoxW(window:*mut std::ffi::c_void,text:*const u16,title:*const u16,flags:u32)->i32;}
        let wide=|s:&str|s.encode_utf16().chain(Some(0)).collect::<Vec<_>>();
        unsafe{MessageBoxW(std::ptr::null_mut(),wide(&text).as_ptr(),wide("RustTD6 update").as_ptr(),0x4|0x40|0x10000)==6}
    };
    #[cfg(not(windows))]
    let open={println!("{text}");false};
    open&&std::process::Command::new(game.join("BloonsTD6.exe")).current_dir(game).spawn().is_ok()
}
#[cfg(test)]
mod tests{
    use super::*;use serde_json::json;
    #[test]
    fn asset_gate_supports_legacy_code_only_and_required_revisions(){
        assert!(!needs_assets(&json!({"build":1}),&Value::Null));
        assert!(needs_assets(&json!({"assets_revision":"new"}),&Value::Null));
        assert!(needs_assets(&json!({"assets_revision":"new"}),&json!({"revision":"old"})));
        assert!(!needs_assets(&json!({"assets_revision":"new"}),&json!({"revision":"new"})));
    }
}
