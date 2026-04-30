use crate::config::{BLOCK_BUFFER_SIZE, GRANULE_SIZE};
use crate::mark::{Mark, MarkReader, MarkWriter};
use std::fs::{File, OpenOptions};
use std::io::{Read, Seek, SeekFrom, Write};

pub struct StringColumn {
    pub data: Vec<String>,
}


pub struct StringColumnWriter {
    col: StringColumn,
    mark_writer: MarkWriter,
    data_file: File,
}

pub struct StringColumnReader {
    mark_reader: MarkReader,
    data_file: File,
    block_cache: Option<(u64, Vec<u8>)>,
}

impl StringColumnWriter {
    pub fn create(data_path: &str, mark_path: &str) -> std::io::Result<Self> {
        let data_file = OpenOptions::new()
            .write(true)
            .create(true)
            .truncate(true)
            .open(data_path)?;

        Ok(StringColumnWriter {
            col: StringColumn { data: Vec::new() },
            data_file: data_file,
            mark_writer: MarkWriter::create(mark_path)?,
        })
    }

    pub fn push(&mut self, val: String) {
        self.col.data.push(val);
    }

    pub fn flush(&mut self) -> std::io::Result<()> {
        let mut buffer: Vec<u8> = Vec::with_capacity(BLOCK_BUFFER_SIZE);
        let mut block_offset: u64 = 0; // File position of current block
        let mut bytes_in_block: usize = 0; // Offset of the current granule within the block
        let mut granule_start: usize = 0; // index of first string in the current granule
        let mut granule_bytes: usize = 0; // byte count of the granule

        for i in 0..self.col.data.len() {
            // add the current string to the granule
            granule_bytes += 4 + self.col.data[i].len();
            let count = i - granule_start + 1;

            // Seal the granule if we hit the count limit/byte size limit
            if count >= GRANULE_SIZE || granule_bytes >= BLOCK_BUFFER_SIZE {
                self.mark_writer.write(&Mark {
                    block_offset,
                    granule_offset: bytes_in_block as u64,
                    num_rows: count as u64,
                });

                // serialize the strings of this granule
                for s in &self.col.data[granule_start..=i] {
                    buffer.extend_from_slice(&(s.len() as i32).to_le_bytes());
                    buffer.extend_from_slice(s.as_bytes());
                }

                bytes_in_block += granule_bytes;

                if buffer.len() >= BLOCK_BUFFER_SIZE {
                    let compressed = lz4_flex::compress_prepend_size(&buffer);
                    self.data_file
                        .write_all(&(compressed.len() as u32).to_le_bytes())?;
                    self.data_file.write_all(&(compressed))?;

                    block_offset += 4 + compressed.len() as u64;
                    buffer.clear();
                    bytes_in_block = 0;
                }

                granule_start = i + 1;
                granule_bytes = 0;
            }
        }
        // Write the remaining partial granule that never hit either limit
        if granule_start < self.col.data.len() {
            let count = self.col.data.len() - granule_start;
            self.mark_writer.write(&Mark {
                block_offset,
                granule_offset: bytes_in_block as u64,
                num_rows: count as u64,
            });
            for s in &self.col.data[granule_start..] {
                buffer.extend_from_slice(&(s.len() as i32).to_le_bytes());
                buffer.extend_from_slice(s.as_bytes());
            }
        }

        // Compress and write whatever is left in the buffer
        if !buffer.is_empty() {
            let compressed = lz4_flex::compress_prepend_size(&buffer);
            self.data_file
                .write_all(&(compressed.len() as u32).to_le_bytes())?;
            self.data_file.write_all(&compressed)?;
        }

        self.mark_writer.flush()
    }
}

impl StringColumnReader {
    pub fn open(data_path: &str, mark_path: &str) -> std::io::Result<Self> {
        Ok(StringColumnReader {
            data_file: File::open(data_path)?,
            mark_reader: MarkReader::open(mark_path)?,
            block_cache: None,
        })
    }

    pub fn read_granule(&mut self, mark: &Mark) -> std::io::Result<StringColumn> {
        let cache_hit =
            matches!(&self.block_cache, Some((offset, _)) if *offset == mark.block_offset);

        if !cache_hit {
            self.data_file.seek(SeekFrom::Start(mark.block_offset))?;
            let mut len_buf = [0u8; 4];
            self.data_file.read_exact(&mut len_buf)?;
            let compressed_len = u32::from_le_bytes(len_buf) as usize;
            let mut compressed = vec![0u8; compressed_len];
            self.data_file.read_exact(&mut compressed)?;
            let bytes = lz4_flex::decompress_size_prepended(&compressed)
                .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))?;
            self.block_cache = Some((mark.block_offset, bytes));
        }

        let decompressed = &self.block_cache.as_ref().unwrap().1;
        let mut cursor = mark.granule_offset as usize;
        let mut result = StringColumn { data: Vec::with_capacity(mark.num_rows as usize) };

        for _ in 0..mark.num_rows {
            let len =
                i32::from_le_bytes(decompressed[cursor..cursor + 4].try_into().unwrap()) as usize;
            cursor += 4;
            let s = String::from_utf8(decompressed[cursor..cursor + len].to_vec())
                .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))?;
            cursor += len;
            result.data.push(s);
        }

        Ok(result)
    }

    pub fn read_all(&mut self) -> std::io::Result<Vec<StringColumn>> {
        let marks = self.mark_reader.read_all()?;
        marks.iter().map(|mark| self.read_granule(mark)).collect()
    }
}
