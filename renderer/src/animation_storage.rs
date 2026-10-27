//! Lossless disk compression only. The staging worker expands bytes before the
//! existing frame cache; rendering, interpolation and simulation stay unchanged.
use std::{io::{self, BufRead}, path::Path};

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
    decode_reader(io::BufReader::with_capacity(1024 * 1024, std::fs::File::open(dir.join(format!("{name}.z")))?), expected)
}

fn invalid() -> io::Error { io::Error::new(io::ErrorKind::InvalidData, "invalid packed animation") }

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
    }
}
