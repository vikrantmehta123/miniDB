use std::fs::{File, OpenOptions};
use std::io::{Read, Write, Seek, SeekFrom};
use std::path::Path;

const CHUNK_SIZE: usize = 1024;

fn write_column(path: &Path, values: &[i64]) -> std::io::Result<()> {
    let mut file = OpenOptions::new()
        .write(true)
        .create(true)
        .truncate(true)
        .open(path)? ;

    for &v in values {
        file.write_all(&v.to_le_bytes())?;
    }
    Ok(())
}

fn read_chunks(path: &Path, chunk_index: usize) -> std::io::Result<Vec<i64>> {
    let mut file = File::open(path)? ;

    let offset = (chunk_index * CHUNK_SIZE * 8) as u64;

    file.seek(SeekFrom::Start(offset))?;

    let mut buf = vec![0u8; CHUNK_SIZE * 8];

    let bytes_read = file.read(&mut buf)?;

    if bytes_read == 0 {
        return Err(std::io::Error::new(
                std::io::ErrorKind::InvalidInput, 
                "Chunk index out of range"
        ));
    }
    
    let values = buf[..bytes_read]
        .chunks_exact(8)
        .map(|b| i64::from_le_bytes(b.try_into().unwrap()))
        .collect();
    
    Ok(values)
}

fn main() -> std::io::Result<()> {
    let path = Path::new("col_i64.bin");
    let data: Vec<i64> = (0..1500).map(|i| i as i64 * 10).collect();

    write_column(path, &data)?;

    println!("Wrote {} values", data.len());

    let chunk0 = read_chunks(path, 0)?;
    let chunk1 = read_chunks(path, 1)?;

    println!("Chunk 0: {} values, first={}, last={}", chunk0.len(), chunk0[0], chunk0[chunk0.len()-1]);
    println!("Chunk 1: {} values, first={}, last={}", chunk1.len(), chunk1[0], chunk1[chunk1.len()-1]);
    
    Ok(())
}
