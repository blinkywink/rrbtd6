//! Author-owned baked models. Imported once; native meshes/animation run in-game.
use std::{path::{Path,PathBuf},collections::HashMap};

pub(crate) fn segments(key:&str)->Option<(&str,&str)>{
    let (owner,asset)=key.strip_prefix("mod:")?.split_once('/')?;
    let valid=|s:&str|!s.is_empty()&&s.len()<=64&&s.bytes().all(|b|b.is_ascii_alphanumeric()||b==b'-'||b==b'_');
    (valid(owner)&&valid(asset)).then_some((owner,asset))
}
pub(crate) fn model(root:&Path,key:&str)->PathBuf{
    if let Some((owner,asset))=segments(key){return crate::modding::directory(root).join(owner).join("assets").join(asset);}
    if key.starts_with("mod:"){return root.join("models/__invalid_mod_asset");}
    root.join("models").join(key)
}
pub(crate) fn tier(root:&Path,base:&str,key:&str)->PathBuf{
    if key.starts_with("mod:"){model(root,key)}else{model(root,base).join("tiers").join(key)}
}
pub(crate) fn projectile(root:&Path,key:&str)->PathBuf{
    if key.starts_with("mod:"){model(root,key)}else{root.join("models/projectiles").join(key)}
}
/// Index names only at load/reload. Meshes and PNGs retain the normal lazy caches.
pub(crate) fn register(root:&Path,index:&mut HashMap<String,String>){
    index.retain(|k,_|!k.starts_with("mod:"));
    for owner in std::fs::read_dir(crate::modding::directory(root)).into_iter().flatten().flatten().take(256){
        for asset in std::fs::read_dir(owner.path().join("assets")).into_iter().flatten().flatten().take(256){
            let key=format!("mod:{}/{}",owner.file_name().to_string_lossy(),asset.file_name().to_string_lossy());
            if segments(&key).is_some()&&asset.path().join("meta.json").is_file(){index.insert(key.clone(),key);}
        }
    }
}
#[cfg(test)]mod tests{
    use super::*;
    #[test]fn model_references_cannot_escape_the_author_assets_folder(){
        for bad in ["mod:../bad","mod:x/../../secret","mod:C:/bad","mod:x/a/b","mod:x/..","mod:/x"]{assert!(segments(bad).is_none());}
        assert_eq!(segments("mod:example/robot"),Some(("example","robot")));
        assert_eq!(model(Path::new("game/local_extracted"),"mod:example/robot"),Path::new("game/mods/example/assets/robot"));
    }
}
