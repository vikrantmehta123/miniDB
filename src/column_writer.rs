use std::fs::File;
use std::io::{self, BufWriter, Seek, Write};
use std::marker::PhantomData;
use std::path::Path;

use crate::config::{BLOCK_BUFFER_SIZE, GRANULE_SIZE};
use crate::data_type::IDataType;
use crate::mark::{Mark, MarkWriter};

pub struct ColumnStats {
    pub rows: u64,
    pub bin_bytes: u64,
}

pub struct ColumnWriter<T: IDataType> {
    bin: BufWriter<File>,
    marks: MarkWriter,

    block_buf: Vec<u8>,
    pending_marks: Vec<Mark>,

    rows_in_current_granule: usize,
    total_rows: u64,

    _phantom: PhantomData<T>,
}

impl<T: IDataType> ColumnWriter<T> {
    pub fn create(part_dir: &Path, col_name: &str) -> io::Result<Self> {
        let bin_path = part_dir.join(format!("{col_name}.bin"));
        let mrk_path = part_dir.join(format!("{col_name}.mrk"));

        let bin = BufWriter::new(File::create(bin_path)?);
        let marks = MarkWriter::create(&mrk_path)?;

        Ok(Self {
            bin,
            marks,
            block_buf: Vec::with_capacity(BLOCK_BUFFER_SIZE * 2),
            pending_marks: Vec::new(),
            rows_in_current_granule: 0,
            total_rows: 0,
            _phantom: PhantomData,
        })
    }

    pub fn write_chunk(&mut self, values: &[T]) -> io::Result<()> {
        for v in values {
            if self.rows_in_current_granule == 0 {
                self.pending_marks.push(Mark {
                    block_offset: 0, // placeholder — patched at flush
                    decompressed_offset: self.block_buf.len() as u64,
                });
            }

            self.block_buf.extend_from_slice(&v.to_le_bytes_vec());
            self.rows_in_current_granule += 1;
            self.total_rows += 1;

            if self.rows_in_current_granule == GRANULE_SIZE {
                self.rows_in_current_granule = 0;
                if self.block_buf.len() >= BLOCK_BUFFER_SIZE {
                    self.flush_block()?;
                }
            }
        }
        Ok(())
    }

    fn flush_block(&mut self) -> io::Result<()> {
        if self.block_buf.is_empty() {
            return Ok(());
        }

        let block_offset = self.bin.stream_position()?;
        let compressed = lz4_flex::compress_prepend_size(&self.block_buf);
        self.bin
            .write_all(&(compressed.len() as u32).to_le_bytes())?;
        self.bin.write_all(&compressed)?;

        for mut mark in self.pending_marks.drain(..) {
            mark.block_offset = block_offset;
            self.marks.write(&mark);
        }

        self.block_buf.clear();
        Ok(())
    }

    pub fn finish(mut self) -> io::Result<ColumnStats> {
        if self.rows_in_current_granule > 0 {
            self.rows_in_current_granule = 0;
        }
        self.flush_block()?;

        self.bin.flush()?;
        let bin_bytes = self.bin.get_ref().metadata()?.len();
        self.marks.flush()?;

        Ok(ColumnStats {
            rows: self.total_rows,
            bin_bytes,
        })
    }
}
