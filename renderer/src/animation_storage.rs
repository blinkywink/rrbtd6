//! Lossless disk compression only. The staging worker expands bytes before the
//! existing frame cache; rendering, interpolation and simulation stay unchanged.
use std::{io::{self, BufRead, Read, Seek}, path::Path};

pub fn read(dir: &Path, name: &str, expected: u64) -> io::Result<Vec<u8>> {
    let path = dir.join(name);
    // A fresh developer export must win over an older packed copy.
    match std::fs::read(&path) {
        Ok(bytes) => {
            if bytes.len() as u64 != expected { return Err(invalid()); }
            return Ok(bytes);
        }
        Err(e) if e.kind() == io::ErrorKind::NotFound => (),
        Err(e) => return Err(e),
    }
    match std::fs::File::open(dir.join(format!("{name}.z"))) {
        Ok(file) => decode_reader(io::BufReader::with_capacity(1024 * 1024, file), expected),
        Err(e) if e.kind() == io::ErrorKind::NotFound => {
            match std::fs::File::open(dir.join(format!("{name}.dz"))) {
                Ok(file)=>decode_delta(io::BufReader::with_capacity(1024*1024,file),expected),
                Err(e) if e.kind()==io::ErrorKind::NotFound=>decode_words(io::BufReader::with_capacity(1024*1024,
                    std::fs::File::open(dir.join(format!("{name}.wz")))?),expected),
                Err(e)=>Err(e),
            }
        }
        Err(e) => Err(e),
    }
}

fn decode_words(mut reader:impl BufRead+Seek,expected:u64)->io::Result<Vec<u8>> {
    let mut header=[0u8;40];reader.read_exact(&mut header)?;
    let mut crc=flate2::Crc::new();crc.update(&header[..36]);
    if &header[..8]!=b"RRWORD01" || crc.sum()!=u32::from_le_bytes(header[36..40].try_into().unwrap()){return Err(invalid());}
    let stride=u64::from_le_bytes(header[8..16].try_into().unwrap());
    let length=u64::from_le_bytes(header[16..24].try_into().unwrap());
    let order=u32::from_le_bytes(header[24..28].try_into().unwrap());
    let packed=u64::from_le_bytes(header[28..36].try_into().unwrap());
    if length!=expected || stride==0 || stride>64*1024*1024 || stride%10!=0 || expected%stride!=0 || order>2{return Err(invalid());}
    let start=reader.stream_position()?;
    let mut decoder=zstd::stream::read::Decoder::with_buffer(reader)?.single_frame();
    decoder.window_log_max(27)?;
    let length=usize::try_from(length).map_err(|_|invalid())?;
    let mut bytes=Vec::new();bytes.try_reserve_exact(length).map_err(|_|invalid())?;bytes.resize(length,0);
    decoder.read_exact(&mut bytes)?;
    if decoder.read(&mut [0u8;1])?!=0{return Err(invalid());}
    let mut reader=decoder.finish();
    if reader.stream_position()?.checked_sub(start)!=Some(packed) || !reader.fill_buf()?.is_empty(){return Err(invalid());}
    // Exact modular reconstruction undoes the offline predictor. Only staging
    // changes: frame caches, quantization and vertex interpolation are untouched.
    let stride=stride as usize;
    let zero=if order==2{vec![0u8;stride]}else{Vec::new()};
        let mut offset=stride;
        while offset<bytes.len(){
            let (before,after)=bytes.split_at_mut(offset);
            if order==0{
                for (d,p) in after[..stride].iter_mut().zip(&before[offset-stride..offset]){*d=d.wrapping_add(*p);}
            }else{
                let older=if order==2{Some(if offset>=2*stride{&before[offset-2*stride..offset-stride]}else{&zero[..]})}else{None};
                restore_word_frame(&mut after[..stride],&before[offset-stride..offset],older);
            }
            offset+=stride;
        }
    Ok(bytes)
}

fn restore_word_frame(current:&mut [u8],previous:&[u8],older:Option<&[u8]>){
    let mut offset=0;
    #[cfg(target_arch="x86_64")]
    {
        use std::arch::x86_64::*;
        // Five vectors cover eight 10-byte records. Word lanes restore the
        // positions; byte lanes independently restore normals and padding.
        const MASKS:[[i16;8];5]=[
            [-1,-1,-1,0,0,-1,-1,-1],[0,0,-1,-1,-1,0,0,-1],
            [-1,-1,0,0,-1,-1,-1,0],[0,-1,-1,-1,0,0,-1,-1],
            [-1,0,0,-1,-1,-1,0,0],
        ];
        // SSE2 is guaranteed by x86-64. Unaligned loads stay within the two
        // disjoint frame slices; the tail below handles every remaining byte.
        unsafe{
            while offset+80<=current.len(){
                for (k,mask) in MASKS.iter().enumerate(){
                    let i=offset+k*16;
                    let a=_mm_loadu_si128(current.as_ptr().add(i).cast());
                    let b=_mm_loadu_si128(previous.as_ptr().add(i).cast());
                    let mask=_mm_loadu_si128(mask.as_ptr().cast());
                    let (words,bytes)=if let Some(older)=older{
                        let c=_mm_loadu_si128(older.as_ptr().add(i).cast());
                        (_mm_add_epi16(a,_mm_sub_epi16(_mm_slli_epi16(b,1),c)),
                         _mm_add_epi8(a,_mm_sub_epi8(_mm_add_epi8(b,b),c)))
                    }else{(_mm_add_epi16(a,b),_mm_add_epi8(a,b))};
                    let value=_mm_or_si128(_mm_and_si128(mask,words),_mm_andnot_si128(mask,bytes));
                    _mm_storeu_si128(current.as_mut_ptr().add(i).cast(),value);
                }
                offset+=80;
            }
        }
    }
    while offset<current.len(){
        if offset%10<6{
            let previous=u16::from_le_bytes([previous[offset],previous[offset+1]]);
            let increment=if let Some(older)=older{previous.wrapping_mul(2).wrapping_sub(u16::from_le_bytes([older[offset],older[offset+1]]))}else{previous};
            let value=u16::from_le_bytes([current[offset],current[offset+1]]).wrapping_add(increment);
            current[offset..offset+2].copy_from_slice(&value.to_le_bytes());
        }else{
            for i in offset..offset+2{
                let increment=if let Some(older)=older{previous[i].wrapping_mul(2).wrapping_sub(older[i])}else{previous[i]};
                current[i]=current[i].wrapping_add(increment);
            }
        }
        offset+=2;
    }
}

fn decode_delta(mut reader: impl BufRead, expected: u64) -> io::Result<Vec<u8>> {
    let mut header = [0u8; 16];
    reader.read_exact(&mut header)?;
    if &header[..8] != b"RRDIF001" { return Err(invalid()); }
    let stride = u64::from_le_bytes(header[8..].try_into().unwrap());
    if stride == 0 || stride > 64 * 1024 * 1024 || expected % stride != 0 { return Err(invalid()); }
    let stride = stride as usize;
    let mut bytes = decode_reader(reader, expected)?;
    // Reconstruct once in place. No extra full-animation buffer or per-frame
    // decompression is needed after staging.
    let mut offset = stride;
    while offset < bytes.len() {
        let (before, after) = bytes.split_at_mut(offset);
        for (delta, previous) in after[..stride].iter_mut().zip(&before[offset-stride..]) {
            *delta = delta.wrapping_add(*previous);
        }
        offset += stride;
    }
    Ok(bytes)
}

fn invalid() -> io::Error { io::Error::new(io::ErrorKind::InvalidData, "invalid packed animation") }

/// Installation-only acceptance gate. Old files remain readable until Python
/// checks file stamps and applies this report. This never runs during gameplay.
pub fn audit_candidates(root:&Path)->io::Result<Vec<serde_json::Value>>{
    let root=root.canonicalize()?;let mut directories=vec![root.clone()];let mut candidates=Vec::new();
    while let Some(dir)=directories.pop(){
        for entry in std::fs::read_dir(&dir)?{
            let entry=entry?;let path=entry.path();let meta=std::fs::symlink_metadata(&path)?;
            #[cfg(windows)]{use std::os::windows::fs::MetadataExt;if meta.file_attributes()&0x400!=0{continue;}}
            if meta.file_type().is_symlink(){continue;}
            if meta.is_dir(){directories.push(path);}else if entry.file_name()=="anim_q.bin.wz"{
                if ["anim_q.bin","anim_q.bin.z","anim_q.bin.dz"].iter().any(|p|dir.join(p).is_file()){candidates.push(dir.clone());}
            }
        }
    }
    candidates.sort();let mut results=Vec::new();
    let mut measured:std::collections::HashMap<(String,blake3::Hash,blake3::Hash),(bool,f64,f64)>=Default::default();
    for (index,dir) in candidates.iter().enumerate(){
        let meta:serde_json::Value=serde_json::from_slice(&std::fs::read(dir.join("meta.json"))?)?;
        let count=meta["anims"].as_array().ok_or_else(invalid)?.iter().try_fold(0u64,|end,a|Some(end.max(a["offset"].as_u64()?.checked_add(a["frames"].as_u64()?)?))).ok_or_else(invalid)?;
        let expected=count.checked_mul(meta["vertex_count"].as_u64().ok_or_else(invalid)?).and_then(|n|n.checked_mul(10)).ok_or_else(invalid)?;
        let packed_path=dir.join("anim_q.bin.wz");
        let new_decode=||decode_words(io::BufReader::with_capacity(1024*1024,std::fs::File::open(&packed_path)?),expected);
        let reference=read(dir,"anim_q.bin",expected)?;
        if new_decode()?!=reference{return Err(invalid());}
        let old_path=["anim_q.bin","anim_q.bin.z","anim_q.bin.dz"].iter().map(|name|dir.join(name)).find(|p|p.is_file()).ok_or_else(invalid)?;
        let key=(old_path.file_name().unwrap().to_string_lossy().into_owned(),
            blake3::hash(&std::fs::read(&old_path)?),blake3::hash(&std::fs::read(&packed_path)?));
        if let Some(&(accepted,old_ms,new_ms))=measured.get(&key){
            results.push(serde_json::json!({"directory":dir.strip_prefix(&root).map_err(|_|invalid())?,
                "accepted":accepted,"old_ms":old_ms,"new_ms":new_ms,"decoded_bytes":expected,"timing_reused":true}));
            continue;
        }
        let mut old_times=Vec::new();let mut new_times=Vec::new();
        // Alternate order to reduce cache/temperature bias. Comparing decoded
        // bytes is outside the timed section; both timings include allocation.
        for trial in 0..3{
            for newer in if trial%2==0{[false,true]}else{[true,false]}{
                let start=std::time::Instant::now();
                let bytes=if newer{new_decode()?}else{read(dir,"anim_q.bin",expected)?};
                let ms=start.elapsed().as_secs_f64()*1000.0;
                if bytes!=reference{return Err(invalid());}
                if newer{new_times.push(ms);}else{old_times.push(ms);}
            }
        }
        old_times.sort_by(f64::total_cmp);new_times.sort_by(f64::total_cmp);
        // Require a 3% margin, rather than accepting an apparent tie/noise.
        let accepted=new_times[1]+0.1<=old_times[1]*0.97;
        measured.insert(key,(accepted,old_times[1],new_times[1]));
        results.push(serde_json::json!({"directory":dir.strip_prefix(&root).map_err(|_|invalid())?,
            "accepted":accepted,"old_ms":old_times[1],"new_ms":new_times[1],"decoded_bytes":expected}));
        if (index+1)%50==0{eprintln!("codec audit {}/{}",index+1,candidates.len());}
    }
    Ok(results)
}

fn decode_reader(mut reader: impl BufRead, expected: u64) -> io::Result<Vec<u8>> {
    let length = usize::try_from(expected).map_err(|_| invalid())?;
    // Bounds come from clip/vertex metadata, not the compressed stream.
    let mut output = Vec::new();
    output.try_reserve_exact(length.checked_add(1).ok_or_else(invalid)?)
        .map_err(|_| invalid())?;
    output.resize(length + 1, 0);
    let mut decoder = flate2::Decompress::new(true);
    loop {
        let input = reader.fill_buf()?;
        let eof = input.is_empty();
        let before_in = decoder.total_in();
        let before_out = decoder.total_out();
        let status = decoder.decompress(input, &mut output[before_out as usize..],
            if eof { flate2::FlushDecompress::Finish } else { flate2::FlushDecompress::None })
            .map_err(|_| invalid())?;
        let consumed = (decoder.total_in() - before_in) as usize;
        reader.consume(consumed);
        if decoder.total_out() > expected { return Err(invalid()); }
        if status == flate2::Status::StreamEnd {
            if decoder.total_out() != expected || !reader.fill_buf()?.is_empty() { return Err(invalid()); }
            output.truncate(length);
            return Ok(output);
        }
        if eof || (consumed == 0 && before_out == decoder.total_out()) { return Err(invalid()); }
    }
}

#[cfg(test)]
fn decode(bytes: &[u8], expected: u64) -> io::Result<Vec<u8>> {
    decode_reader(io::Cursor::new(bytes), expected)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;

    fn word_packed(source:&[u8],stride:usize,order:u32)->Vec<u8>{
        let mut filtered=source.to_vec();
        for _ in 0..order{for offset in (1..source.len()/stride).rev().map(|f|f*stride){
            let (before,after)=filtered.split_at_mut(offset);
            for (current,previous) in after[..stride].chunks_exact_mut(10).zip(before[offset-stride..].chunks_exact(10)){
                for k in [0,2,4]{
                    let v=u16::from_le_bytes([current[k],current[k+1]]).wrapping_sub(u16::from_le_bytes([previous[k],previous[k+1]]));
                    current[k..k+2].copy_from_slice(&v.to_le_bytes());
                }
                for k in 6..10{current[k]=current[k].wrapping_sub(previous[k]);}
            }
        }}
        let mut encoder=zstd::stream::write::Encoder::new(Vec::new(),3).unwrap();encoder.include_checksum(true).unwrap();
        encoder.write_all(&filtered).unwrap();let compressed=encoder.finish().unwrap();
        let mut bytes=b"RRWORD01".to_vec();bytes.extend_from_slice(&(stride as u64).to_le_bytes());
        bytes.extend_from_slice(&(source.len() as u64).to_le_bytes());bytes.extend_from_slice(&order.to_le_bytes());
        bytes.extend_from_slice(&(compressed.len() as u64).to_le_bytes());
        let mut crc=flate2::Crc::new();crc.update(&bytes);bytes.extend_from_slice(&crc.sum().to_le_bytes());bytes.extend(compressed);bytes
    }

    #[test]fn vector_word_restoration_matches_scalar_at_every_tail(){
        for stride in (10..=400).step_by(10){
            let mut a:Vec<u8>=(0..stride).map(|i|((i*71+193)%256) as u8).collect();
            let b:Vec<u8>=(0..stride).map(|i|((i*113+251)%256) as u8).collect();
            let mut expected=a.clone();
            for (current,previous) in expected.chunks_exact_mut(10).zip(b.chunks_exact(10)){
                for k in [0,2,4]{let v=u16::from_le_bytes([current[k],current[k+1]]).wrapping_add(u16::from_le_bytes([previous[k],previous[k+1]]));current[k..k+2].copy_from_slice(&v.to_le_bytes());}
                for k in 6..10{current[k]=current[k].wrapping_add(previous[k]);}
            }
            restore_word_frame(&mut a,&b,None);assert_eq!(a,expected,"stride {stride}");
        }
    }

    #[test]fn native_audit_reuses_decisions_and_keeps_sources_readable(){
        let dir=std::env::temp_dir().join(format!("rrbtd6-codec-audit-{}-{}",std::process::id(),std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_nanos()));
        let source:Vec<u8>=(0..10000).map(|i|((i*71+i/100*137)%256) as u8).collect();
        let mut delta=source.clone();for i in (100..delta.len()).rev(){delta[i]=delta[i].wrapping_sub(delta[i-100]);}
        let mut old=b"RRDIF001".to_vec();old.extend_from_slice(&100u64.to_le_bytes());old.extend(packed(&delta));
        let new=word_packed(&source,100,2);
        for name in ["a","b"]{
            let path=dir.join(name);std::fs::create_dir_all(&path).unwrap();
            std::fs::write(path.join("anim_q.bin.dz"),&old).unwrap();std::fs::write(path.join("anim_q.bin.wz"),&new).unwrap();
            std::fs::write(path.join("meta.json"),br#"{"vertex_count":10,"anims":[{"offset":0,"frames":100}]}"#).unwrap();
        }
        let results=audit_candidates(&dir).unwrap();assert_eq!(results.len(),2);
        assert_eq!(results[0]["accepted"],results[1]["accepted"]);
        assert_eq!(results[0]["old_ms"],results[1]["old_ms"]);
        assert_eq!(results[0]["new_ms"],results[1]["new_ms"]);
        assert_eq!(results[1]["timing_reused"],true);
        assert_eq!(read(&dir.join("a"),"anim_q.bin",10000).unwrap(),source);
        assert_eq!(read(&dir.join("b"),"anim_q.bin",10000).unwrap(),source);
        assert!(dir.starts_with(std::env::temp_dir()));std::fs::remove_dir_all(dir).unwrap();
    }

    #[test]fn word_prediction_restores_every_byte_and_rejects_corruption(){
        let source:Vec<u8>=(0..10000).map(|i|((i*71+i/100*137)%256) as u8).collect();
        for order in [1,2]{
            let bytes=word_packed(&source,100,order);
            assert_eq!(decode_words(io::Cursor::new(&bytes),source.len() as u64).unwrap(),source);
            assert_eq!(decode_words(io::BufReader::with_capacity(7,io::Cursor::new(&bytes)),source.len() as u64).unwrap(),source);
            for offset in [0,8,16,24,28,36,bytes.len()-1]{
                let mut bad=bytes.clone();bad[offset]^=1;assert!(decode_words(io::Cursor::new(&bad),source.len() as u64).is_err());
            }
            assert!(decode_words(io::Cursor::new(&bytes[..bytes.len()-1]),source.len() as u64).is_err());
            let mut bad=bytes.clone();bad.push(0);assert!(decode_words(io::Cursor::new(&bad),source.len() as u64).is_err());
            assert!(decode_words(io::Cursor::new(&bytes),source.len() as u64+1).is_err());
        }
    }

    #[test]
    #[ignore="requires a local old/new word-animation fixture"]
    fn local_word_decode_benchmark(){
        let dir=std::env::var_os("RRBTD6_WORD_FIXTURE").unwrap();let dir=Path::new(&dir);
        let old=std::fs::read(dir.join("anim_q.bin.dz")).unwrap();
        let new=std::fs::read(dir.join("anim_q.bin.wz")).unwrap();
        let meta:serde_json::Value=serde_json::from_slice(&std::fs::read(dir.join("meta.json")).unwrap()).unwrap();
        let count=meta["anims"].as_array().unwrap().iter().map(|a|a["offset"].as_u64().unwrap()+a["frames"].as_u64().unwrap()).max().unwrap();
        let expected=count*meta["vertex_count"].as_u64().unwrap()*10;
        let source=decode_delta(io::Cursor::new(&old),expected).unwrap();
        let start=std::time::Instant::now();
        for _ in 0..10{assert_eq!(decode_delta(io::Cursor::new(&old),expected).unwrap(),source);}
        let old_ms=start.elapsed().as_secs_f64()*100.0;
        let start=std::time::Instant::now();
        for _ in 0..10{assert_eq!(decode_words(io::Cursor::new(&new),expected).unwrap(),source);}
        let new_ms=start.elapsed().as_secs_f64()*100.0;
        eprintln!("word-storage: {}: {} -> {} bytes; native decode {:.2} -> {:.2} ms; all {} bytes exact",dir.display(),old.len(),new.len(),old_ms,new_ms,expected);
    }

    fn packed(bytes: &[u8]) -> Vec<u8> {
        let mut e = flate2::write::ZlibEncoder::new(Vec::new(), flate2::Compression::fast());
        e.write_all(bytes).unwrap(); e.finish().unwrap()
    }
    #[test]
    fn payload_is_exact_and_bad_streams_fail() {
        let source: Vec<_> = (0..200_000).map(|i| (i % 251) as u8).collect();
        let packed = packed(&source);
        assert_eq!(decode(&packed, source.len() as u64).unwrap(), source);
        assert_eq!(decode_reader(io::BufReader::with_capacity(7, io::Cursor::new(&packed)), source.len() as u64).unwrap(), source);
        for len in [0, 1, source.len() as u64 - 1, source.len() as u64 + 1] {
            assert!(decode(&packed, len).is_err());
        }
        assert!(decode(&packed[..packed.len()-1], source.len() as u64).is_err());
        let mut corrupt = packed.clone(); let last = corrupt.len()-1; corrupt[last] ^= 1;
        assert!(decode(&corrupt, source.len() as u64).is_err());
        let mut trailing = packed; trailing.push(0);
        assert!(decode(&trailing, source.len() as u64).is_err());
    }
    #[test]
    fn temporal_prediction_restores_exact_bytes_including_wraparound() {
        let stride = 57;
        let source: Vec<u8> = (0..stride*23).map(|i| ((i*i+127*i)%256) as u8).collect();
        let mut delta = source.clone();
        for i in stride..source.len() { delta[i] = source[i].wrapping_sub(source[i-stride]); }
        let mut encoded = b"RRDIF001".to_vec();
        encoded.extend_from_slice(&(stride as u64).to_le_bytes());
        encoded.extend_from_slice(&packed(&delta));
        assert_eq!(decode_delta(io::Cursor::new(&encoded),source.len() as u64).unwrap(),source);
        assert_eq!(decode_delta(io::BufReader::with_capacity(7,io::Cursor::new(&encoded)),source.len() as u64).unwrap(),source);
        assert!(decode_delta(io::Cursor::new(&encoded),source.len() as u64-1).is_err());
        let mut wrong=encoded.clone();wrong[0]=0;
        assert!(decode_delta(io::Cursor::new(&wrong),source.len() as u64).is_err());
        wrong=encoded.clone();wrong[8..16].fill(0);
        assert!(decode_delta(io::Cursor::new(&wrong),source.len() as u64).is_err());
        assert!(decode_delta(io::Cursor::new(&encoded[..encoded.len()-1]),source.len() as u64).is_err());
    }

    #[test]
    fn new_unpacked_export_overrides_stale_packed_copy() {
        let dir = std::env::temp_dir().join(format!("rrbtd6-storage-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(dir.join("anim_q.bin.z"), packed(b"old")).unwrap();
        assert_eq!(read(&dir,"anim_q.bin",3).unwrap(), b"old");
        std::fs::write(dir.join("anim_q.bin"), b"new").unwrap();
        assert_eq!(read(&dir,"anim_q.bin",3).unwrap(), b"new");
        assert!(read(&dir,"anim_q.bin",4).is_err());
        std::fs::remove_dir_all(dir).unwrap();
    }
    #[test]
    #[ignore = "requires a local packed-animation benchmark fixture"]
    fn local_decode_benchmark() {
        let dir = std::env::var_os("RRBTD6_STORAGE_FIXTURE").unwrap();
        let dir = Path::new(&dir);
        let source = std::fs::read(dir.join("anim_q.bin")).unwrap();
        let compressed = std::fs::read(dir.join("anim_q.bin.z")).unwrap();
        let start = std::time::Instant::now();
        for _ in 0..10 { assert_eq!(decode(&compressed, source.len() as u64).unwrap(), source); }
        eprintln!("{} -> {} bytes; exact decode average {:.2} ms", source.len(), compressed.len(), start.elapsed().as_secs_f64()*100.0);
        if let Ok(bytes) = std::fs::read(dir.join("anim_q.bin.dz")) {
            let start = std::time::Instant::now();
            for _ in 0..10 { assert_eq!(decode_delta(io::Cursor::new(&bytes),source.len() as u64).unwrap(), source); }
            eprintln!("predicted: {} bytes; exact decode average {:.2} ms", bytes.len(),start.elapsed().as_secs_f64()*100.0);
        }
    }
}
