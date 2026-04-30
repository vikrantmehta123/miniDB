use std::fs::{File, OpenOptions};
use std::io::{Read, Write};

pub struct Mark {
    pub block_offset: u64,
    pub granule_offset: u64, 
    pub num_rows: u64,
}


impl Mark {
    pub fn to_bytes(&self) -> [u8; 24] {
        let mut buf = [0u8; 24];
        buf[0..8].copy_from_slice(&self.block_offset.to_le_bytes());
        buf[8..16].copy_from_slice(&self.granule_offset.to_le_bytes());
        buf[16..24].copy_from_slice(&self.num_rows.to_le_bytes());
        buf
    }

    pub fn from_bytes(bytes: &[u8]) -> Mark {
        let block_offset = u64::from_le_bytes(bytes[0..8].try_into().unwrap());
        let granule_offset = u64::from_le_bytes(bytes[8..16].try_into().unwrap());
        let num_rows = u64::from_le_bytes(bytes[16..24].try_into().unwrap());

        Mark { block_offset, granule_offset, num_rows }
    }
}


pub struct MarkWriter {
    file: File,
    buf: Vec<u8>,
}

impl MarkWriter {
    pub fn create(path: &str) -> std::io::Result<Self> {
        let file = OpenOptions::new()
            .write(true)
            .create(true)
            .truncate(true)
            .open(path)?;

        Ok(MarkWriter{file, buf: Vec::new()})
    }

    pub fn write(&mut self, mark: &Mark) {
        self.buf.extend_from_slice(&mark.to_bytes());
    }

    pub fn flush(&mut self) -> std::io::Result<()> {
        self.file.write_all(&self.buf)?;
        Ok(())
    }
}

pub struct MarkReader {
    file: File
}

impl MarkReader {
    pub fn open(path: &str) -> std::io::Result<Self> {

        Ok(MarkReader { file: File::open(path)? })
    }

    pub fn read_all(&mut self) -> std::io::Result<Vec<Mark>> {
        let mut buf = Vec::new();
        self.file.read_to_end(&mut buf)?;
        Ok(buf.chunks_exact(24).map(Mark::from_bytes).collect())
    }
}



