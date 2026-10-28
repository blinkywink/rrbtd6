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
    match std::fs::File::open(dir.join(format!("{name}.z"))) {
        Ok(file) => decode_reader(io::BufReader::with_capacity(1024 * 1024, file), expected),
        Err(e) if e.kind() == io::ErrorKind::NotFound => {
            decode_delta(io::BufReader::with_capacity(1024 * 1024,
                std::fs::File::open(dir.join(format!("{name}.dz")))?), expected)
        }
        Err(e) => Err(e),
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
