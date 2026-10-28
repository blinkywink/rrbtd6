//! Offline conversion of legacy float snapshots to the exact representation the
//! runtime already samples. Invoked before window creation, never during play.
use std::{fs, io::{self, Read, Write}, path::Path, time::SystemTime};

struct Pending(std::path::PathBuf);
impl Drop for Pending { fn drop(&mut self) { let _ = fs::remove_file(&self.0); } }

fn invalid(message:&str)->io::Error {io::Error::new(io::ErrorKind::InvalidData,message)}
fn stamp(path:&Path)->io::Result<(u64,SystemTime)>{let m=fs::metadata(path)?;Ok((m.len(),m.modified()?))}
fn linked(path:&Path)->io::Result<bool>{
    let m=fs::symlink_metadata(path)?;
    #[cfg(windows)] {
        use std::os::windows::fs::MetadataExt;
        return Ok(m.file_type().is_symlink() || m.file_attributes()&0x400!=0);
    }
    #[cfg(not(windows))] {Ok(m.file_type().is_symlink())}
}
fn pending(dir:&Path,name:&str)->io::Result<(Pending,fs::File)>{
    let nonce=SystemTime::now().duration_since(SystemTime::UNIX_EPOCH).unwrap().as_nanos();
    let p=dir.join(format!("{name}.runtime-{}-{nonce}.pending",std::process::id()));
    let f=fs::OpenOptions::new().create_new(true).write(true).open(&p)?;
    Ok((Pending(p),f))
}

fn convert(dir:&Path)->io::Result<Option<(u64,u64)>> {
    let meta_path=dir.join("meta.json");
    if !meta_path.is_file(){return Ok(None);}
    if linked(&meta_path)? {return Err(invalid("linked animation metadata"));}
    let mut meta:serde_json::Value=serde_json::from_slice(&fs::read(&meta_path)?)?;
    if meta.get("quant").is_some() {return Ok(None);}
    let Some(anims)=meta["anims"].as_array().filter(|a|!a.is_empty()) else{return Ok(None)};
    let count=anims.iter().try_fold(0u64,|end,a|Some(end.max(a["offset"].as_u64()?.checked_add(a["frames"].as_u64()?)?)))
        .ok_or_else(||invalid("invalid animation clips"))?;
    let nverts=meta["vertex_count"].as_u64().ok_or_else(||invalid("missing animation vertex count"))?;
    let expected=count.checked_mul(nverts).and_then(|n|n.checked_mul(24)).ok_or_else(||invalid("animation too large"))?;
    let Some(source_path)=["anim.bin","anim.bin.z","anim.bin.dz"].into_iter().map(|n|dir.join(n)).find(|p|p.is_file()) else{return Ok(None)};
    if linked(&source_path)? {return Err(invalid("linked source animation"));}
    let source_stamp=stamp(&source_path)?;
    let meta_stamp=stamp(&meta_path)?;
    let source=crate::animation_storage::read(dir,"anim.bin",expected)?;
    let (q_pending,mut file)=pending(dir,"anim_q.bin")?;
    let (lo,hi,expected_hash)=crate::anim::write_runtime_quantized(&source,&mut file)?;
    file.flush()?;file.sync_all()?;drop(file);drop(source);
    let q_size=expected/24*10;
    let mut hash=blake3::Hasher::new();
    let mut file=fs::File::open(&q_pending.0)?;
    let mut buffer=vec![0u8;1024*1024];let mut total=0;
    loop{let n=file.read(&mut buffer)?;if n==0{break;}total+=n as u64;hash.update(&buffer[..n]);}
    drop(file);
    if total!=q_size || hash.finalize()!=expected_hash {return Err(invalid("quantized animation verification failed"));}
    meta["quant"]=serde_json::json!({"min":lo,"max":hi});
    meta["runtime_quantizer"]=serde_json::json!(1);
    let (meta_pending,mut file)=pending(dir,"meta.json")?;
    file.write_all(&serde_json::to_vec(&meta)?)?;file.flush()?;file.sync_all()?;drop(file);
    if stamp(&meta_path)?!=meta_stamp || stamp(&source_path)?!=source_stamp {return Err(invalid("animation changed during conversion"));}
    let q_target=dir.join("anim_q.bin");
    if q_target.exists() && linked(&q_target)? {return Err(invalid("linked quantized animation"));}
    fs::rename(&q_pending.0,q_target)?;
    // Until metadata switches, the old reader still uses the intact float file.
    fs::rename(&meta_pending.0,&meta_path)?;
    if stamp(&source_path)?==source_stamp {fs::remove_file(source_path)?;}
    Ok(Some((expected,q_size)))
}

pub fn run(root:&Path)->io::Result<()> {
    if linked(root)? {return Err(invalid("linked model root"));}
    let root=fs::canonicalize(root)?;
    let mut directories=vec![root];let mut count=0;let mut before=0;let mut after=0;
    while let Some(dir)=directories.pop(){
        for entry in fs::read_dir(&dir)? {
            let p=entry?.path();if !linked(&p)? && p.is_dir(){directories.push(p);}
        }
        if let Some((b,a))=convert(&dir)?{
            count+=1;before+=b;after+=a;
            println!("runtime representation {count}: {:.3} -> {:.3} GB decoded",before as f64/1e9,after as f64/1e9);
        }
    }
    println!("{count} float models converted to the exact existing runtime quantization");
    Ok(())
}

#[cfg(test)]mod tests{
    use super::*;
    #[test]fn conversion_preserves_clips_and_reuses_runtime_precision(){
        let dir=std::env::temp_dir().join(format!("rrbtd6-runtime-export-{}",std::process::id()));
        fs::create_dir_all(&dir).unwrap();
        let source:Vec<u8>=(0..60).flat_map(|i|(i as f32*0.17-3.0).to_le_bytes()).collect();
        fs::write(dir.join("anim.bin"),&source).unwrap();
        let meta=serde_json::json!({"vertex_count":2,"anims":[{"offset":0,"frames":5,"name":"fixture","loop":true}]});
        fs::write(dir.join("meta.json"),serde_json::to_vec(&meta).unwrap()).unwrap();
        assert_eq!(convert(&dir).unwrap(),Some((240,100)));
        let converted:serde_json::Value=serde_json::from_slice(&fs::read(dir.join("meta.json")).unwrap()).unwrap();
        assert_eq!(converted["anims"],meta["anims"]);
        let mut expected=Vec::new();let (lo,hi,_)=crate::anim::write_runtime_quantized(&source,&mut expected).unwrap();
        assert_eq!(fs::read(dir.join("anim_q.bin")).unwrap(),expected);
        assert_eq!(converted["quant"]["min"],serde_json::json!(lo));
        assert_eq!(converted["quant"]["max"],serde_json::json!(hi));
        assert!(!dir.join("anim.bin").exists());assert_eq!(convert(&dir).unwrap(),None);
        fs::remove_dir_all(dir).unwrap();
    }
    #[test]fn malformed_source_leaves_original_and_metadata_intact(){
        let dir=std::env::temp_dir().join(format!("rrbtd6-runtime-bad-{}",std::process::id()));
        fs::create_dir_all(&dir).unwrap();
        let meta=br#"{"vertex_count":2,"anims":[{"offset":0,"frames":5}]}"#;
        fs::write(dir.join("meta.json"),meta).unwrap();fs::write(dir.join("anim.bin"),b"bad").unwrap();
        assert!(convert(&dir).is_err());
        assert_eq!(fs::read(dir.join("meta.json")).unwrap(),meta);
        assert_eq!(fs::read(dir.join("anim.bin")).unwrap(),b"bad");
        assert!(!dir.join("anim_q.bin").exists());
        fs::remove_dir_all(dir).unwrap();
    }
    #[test]fn packed_float_sources_are_converted_without_a_window(){
        for delta in [false,true]{
            let dir=std::env::temp_dir().join(format!("rrbtd6-runtime-packed-{}-{delta}",std::process::id()));
            fs::create_dir_all(&dir).unwrap();
            let source:Vec<u8>=(0..60).flat_map(|i|(i as f32*0.13).to_le_bytes()).collect();
            let mut payload=source.clone();
            if delta{for i in 48..payload.len(){payload[i]=source[i].wrapping_sub(source[i-48]);}}
            let mut encoder=flate2::write::ZlibEncoder::new(Vec::new(),flate2::Compression::fast());
            encoder.write_all(&payload).unwrap();let mut packed=encoder.finish().unwrap();
            if delta{let mut header=b"RRDIF001".to_vec();header.extend_from_slice(&48u64.to_le_bytes());header.extend_from_slice(&packed);packed=header;}
            let name=if delta{"anim.bin.dz"}else{"anim.bin.z"};
            fs::write(dir.join(name),packed).unwrap();
            fs::write(dir.join("meta.json"),br#"{"vertex_count":2,"anims":[{"offset":0,"frames":5}]}"#).unwrap();
            assert_eq!(convert(&dir).unwrap(),Some((240,100)));
            let mut expected=Vec::new();crate::anim::write_runtime_quantized(&source,&mut expected).unwrap();
            assert_eq!(fs::read(dir.join("anim_q.bin")).unwrap(),expected);
            assert!(!dir.join(name).exists());fs::remove_dir_all(dir).unwrap();
        }
    }
}
