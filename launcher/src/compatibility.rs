//! Asset compatibility is decided before replacing a working executable.
use serde_json::Value;
pub fn needs_assets(release:&Value,assets:&Value)->bool{
    release["assets_revision"].as_str().is_some_and(|required|assets["revision"].as_str()!=Some(required))
}
/// A new build needs freshly extracted game data: update now (the installed version stays until
/// it succeeds), or play the installed version and update next time.
pub fn update_now(build:u64)->bool{
    crate::ui::ask("A RustTD6 update is ready",
        &format!("Build {build} needs to read your BTD6 files again, which takes a few minutes. Your current version stays playable until it finishes.\n\nCo-op needs everyone on the same version."),
        &["Update now","Play now, update later"])==0
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
