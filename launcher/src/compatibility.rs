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

/// The BTD6 version this RustTD6 was matched against (BTD6_VERSION at the repository root; whoever
/// matches a new BTD6 update raises it).
pub const SUPPORTED_BTD6:&str=include_str!("../../BTD6_VERSION");

/// The installed BTD6's version ("57.4"): PlayerSettings.bundleVersion in globalgamemanagers, the
/// highest of the run of length-prefixed version strings after the product name (as
/// tools/assets/export_game_version.py reads it).
pub fn installed_btd6(game:&std::path::Path)->Option<String>{
    let data=std::fs::read(game.join("BloonsTD6_Data/globalgamemanagers")).ok()?;
    bundle_version(&data)
}

fn bundle_version(data:&[u8])->Option<String>{
    let product=data.windows(9).position(|w|w==b"BloonsTD6").unwrap_or(0);
    let is_version=|s:&[u8]|{
        let parts:Vec<&[u8]>=s.split(|&c|c==b'.').collect();
        parts.len()>=2&&parts.iter().all(|p|!p.is_empty()&&p.iter().all(u8::is_ascii_digit))
    };
    let (mut run,mut i,mut last_end):(Vec<String>,usize,usize)=(Vec::new(),product,0);
    while i+4<=data.len(){
        let n=data[i] as usize;
        if (3..=16).contains(&n)&&data[i+1..i+4]==[0,0,0]&&i+4+n<=data.len()&&is_version(&data[i+4..i+4+n]){
            if !run.is_empty()&&i-last_end>4{break;}
            run.push(String::from_utf8_lossy(&data[i+4..i+4+n]).into_owned());
            last_end=i+4+n;i=last_end;continue;
        }
        i+=1;
    }
    run.into_iter().max_by_key(|v|numbers(v))
}

fn numbers(v:&str)->Vec<u32>{v.trim().split('.').map(|p|p.parse().unwrap_or(0)).collect()}

/// Major.minor only: a hotfix build of the same update counts as the same version.
pub fn compare_btd6(installed:&str,supported:&str)->std::cmp::Ordering{
    let key=|v:&str|{let n=numbers(v);(n.first().copied().unwrap_or(0),n.get(1).copied().unwrap_or(0))};
    key(installed).cmp(&key(supported))
}

#[cfg(test)]
mod tests{
    use super::*;use serde_json::json;
    fn prefixed(s:&str)->Vec<u8>{let mut v=vec![s.len() as u8,0,0,0];v.extend(s.as_bytes());v}
    #[test]
    fn bundle_version_is_the_highest_of_the_run_after_the_product_name(){
        let mut d=b"junk 9.9 BloonsTD6\0\0\0\0".to_vec();
        for s in ["1.0","1.0","57.4"]{d.extend(prefixed(s));while d.len()%4!=0{d.push(0);}}
        d.extend(b"\0\0\0\0\0\0\0\0\0\0\0\0");d.extend(prefixed("99.0"));
        assert_eq!(bundle_version(&d).as_deref(),Some("57.4"));
        assert_eq!(bundle_version(b"BloonsTD6 nothing here"),None);
    }
    #[test]
    fn versions_compare_by_update_not_hotfix(){
        use std::cmp::Ordering::*;
        assert_eq!(compare_btd6("57.4","57.4\n"),Equal);
        assert_eq!(compare_btd6("57.4.1","57.4"),Equal);
        assert_eq!(compare_btd6("56.2","57.4"),Less);
        assert_eq!(compare_btd6("57.10","57.4"),Greater);
        assert_eq!(compare_btd6("58.0","57.4"),Greater);
    }
    #[test]
    fn installed_steam_copy_reads_when_present(){
        let game=std::path::Path::new("C:/Program Files (x86)/Steam/steamapps/common/BloonsTD6");
        if game.join("BloonsTD6_Data/globalgamemanagers").is_file(){
            let v=installed_btd6(game).expect("version");
            assert!(numbers(&v).len()>=2,"{v}");
        }
    }
    #[test]
    fn asset_gate_supports_legacy_code_only_and_required_revisions(){
        assert!(!needs_assets(&json!({"build":1}),&Value::Null));
        assert!(needs_assets(&json!({"assets_revision":"new"}),&Value::Null));
        assert!(needs_assets(&json!({"assets_revision":"new"}),&json!({"revision":"old"})));
        assert!(!needs_assets(&json!({"assets_revision":"new"}),&json!({"revision":"new"})));
    }
}
