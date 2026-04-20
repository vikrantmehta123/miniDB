use std::fs::{File, OpenOptions};
use std::io::{Read, Write, Seek, SeekFrom};
use std::path::{Path, PathBuf};

const CHUNK_SIZE: usize = 1024;

pub struct Column {
    path: PathBuf,
    pub num_rows: usize,
}

impl Column {
    pub fn open(path: &Path, num_rows: usize) -> Self {
        Column { path: path.to_path_buf(), num_rows}
    }

    pub fn append_chunk(&mut self, values: &[i64]) -> std::io::Result<()> {
       let mut file = OpenOptions::new()
           .write(true)
           .create(true)
           .append(true)
           .open(&self.path)?; 

       for &v in values {
            file.write_all(&v.to_le_bytes())?;
       }

       self.num_rows += values.len();
       Ok(())
    }

    pub fn read_chunk(&self, idx: usize) -> std::io::Result<Option<Vec<i64>>> {
        if idx * CHUNK_SIZE >= self.num_rows {
            return Ok(None);
        }

        let mut file = File::open(&self.path)?;


        let offset = (idx*CHUNK_SIZE*8) as u64;

        file.seek(SeekFrom::Start(offset))?;

        let rows_in_chunk = (self.num_rows - idx * CHUNK_SIZE).min(CHUNK_SIZE);

        let mut buf = vec![0u8; rows_in_chunk * 8];
    
        file.read_exact(&mut buf)?;

        let values = buf
            .chunks_exact(8)
            .map(|b| i64::from_le_bytes(b.try_into().unwrap()))
            .collect();

        Ok(Some(values))
        
    }
}
