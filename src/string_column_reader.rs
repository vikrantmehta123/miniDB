use std::fs::File;
use std::io::{self, Read, Seek, SeekFrom};
use std::path::Path;

use crate::mark::{Mark, MarkReader};

pub struct StringColumnReader {
    bin: File,
    marks: Vec<Mark>,
    cache: Option<(u64, Vec<u8>)>,
}

impl StringColumnReader {
    pub fn open(part_dir: &Path, col_name: &str) -> io::Result<Self> {
        let bin_path = part_dir.join(format!("{col_name}.bin"));
        let mrk_path = part_dir.join(format!("{col_name}.mrk"));

        let bin = File::open(bin_path)?;
        let marks = MarkReader::open(&mrk_path)?.read_all()?;

        Ok(Self { bin, marks, cache: None })
    }

    pub fn granule_count(&self) -> usize {
        self.marks.len()
    }

    pub fn read_granule(&mut self, idx: usize) -> io::Result<Vec<String>> {
        let mark = &self.marks[idx];

        // Ensure the right block is cached.
        let cache_hit = matches!(&self.cache, Some((off, _)) if *off == mark.block_offset);
        if !cache_hit {
            self.bin.seek(SeekFrom::Start(mark.block_offset))?;

            let mut len_buf = [0u8; 4];
            self.bin.read_exact(&mut len_buf)?;
            let compressed_len = u32::from_le_bytes(len_buf) as usize;

            let mut compressed = vec![0u8; compressed_len];
            self.bin.read_exact(&mut compressed)?;

            let bytes = lz4_flex::decompress_size_prepended(&compressed)
                .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))?;

            self.cache = Some((mark.block_offset, bytes));
        }

        let block = &self.cache.as_ref().unwrap().1;

        // Granule's byte range inside the decompressed block:
        //   end = next mark's offset if same block, else end of block.
        let start = mark.decompressed_offset as usize;
        let end = match self.marks.get(idx + 1) {
            Some(next) if next.block_offset == mark.block_offset => {
                next.decompressed_offset as usize
            }
            _ => block.len(),
        };

        let mut out = Vec::new();
        let mut cursor = start;
        while cursor < end {
            let len = i32::from_le_bytes(
                block[cursor..cursor + 4]
                    .try_into()
                    .expect("4-byte slice"),
            ) as usize;
            cursor += 4;

            let s = std::str::from_utf8(&block[cursor..cursor + len])
                .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))?
                .to_owned();
            cursor += len;

            out.push(s);
        }
        Ok(out)
    }

    pub fn read_all(&mut self) -> io::Result<Vec<String>> {
        let mut out = Vec::new();
        for i in 0..self.marks.len() {
            out.extend(self.read_granule(i)?);
        }
        Ok(out)
    }
}
