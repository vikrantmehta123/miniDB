use std::fs;
use std::io;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU32, Ordering};

use rayon::prelude::*;

use crate::column_chunk::{ColumnChunk};
use crate::column_writer::{ColumnStats, ColumnWriter};
use crate::data_type::IDataType;
use crate::schema::{ColumnDef, DataType, TableDef};
use crate::string_column_writer::StringColumnWriter;

pub struct PartMetadata {
    pub part_id: u32,
    pub rows: u64,
    pub columns: Vec<ColumnStats>,
}

pub struct TableWriter {
    schema: TableDef,
    table_dir: PathBuf,
    next_part_id: AtomicU32,
}

impl TableWriter {
    pub fn open(table_dir: PathBuf) -> io::Result<Self> {
        let schema = TableDef::open(&table_dir)?;
        let next_part_id = scan_next_part_id(&table_dir)?;
        Ok(Self {
            schema,
            table_dir,
            next_part_id: AtomicU32::new(next_part_id),
        })
    }

    pub fn insert(&self, chunks: Vec<ColumnChunk>) -> io::Result<PartMetadata> {
        // ---- 1. Validate shape & types up front, before any I/O. ----
        if chunks.len() != self.schema.columns.len() {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                format!(
                    "expected {} chunks, got {}",
                    self.schema.columns.len(),
                    chunks.len()
                ),
            ));
        }
        let row_count = chunks.first().map(|c| c.len()).unwrap_or(0);
        for (i, chunk) in chunks.iter().enumerate() {
            if chunk.len() != row_count {
                return Err(io::Error::new(
                    io::ErrorKind::InvalidInput,
                    format!("column {i}: row count mismatch"),
                ));
            }
            check_type(chunk, &self.schema.columns[i])?;
        }

        // ---- 2. Reserve a part id and create a tmp dir. ----
        let part_id = self.next_part_id.fetch_add(1, Ordering::SeqCst);
        let tmp_dir = self.table_dir.join(format!("tmp_part_{:05}", part_id));
        let final_dir = TableDef::part_dir(&self.table_dir, part_id);
        fs::create_dir_all(&tmp_dir)?;

        // ---- 3. Write all columns in parallel. ----
        let result: io::Result<Vec<ColumnStats>> = chunks
            .par_iter()
            .zip(self.schema.columns.par_iter())
            .map(|(chunk, col)| write_one_column(&tmp_dir, col, chunk))
            .collect();

        let stats = match result {
            Ok(s) => s,
            Err(e) => {
                let _ = fs::remove_dir_all(&tmp_dir); // best-effort cleanup
                return Err(e);
            }
        };

        // ---- 4. Atomic-ish finalize: rename tmp -> part_NNNNN. ----
        fs::rename(&tmp_dir, &final_dir)?;
        fs::File::open(&self.table_dir)?.sync_all()?;

        Ok(PartMetadata {
            part_id,
            rows: row_count as u64,
            columns: stats,
        })
    }
}

fn check_type(chunk: &ColumnChunk, col: &ColumnDef) -> io::Result<()> {
    let ok = matches!(
        (chunk, &col.data_type),
        (ColumnChunk::I8(_),   DataType::I8)
        | (ColumnChunk::I16(_),  DataType::I16)
        | (ColumnChunk::I32(_),  DataType::I32)
        | (ColumnChunk::I64(_),  DataType::I64)
        | (ColumnChunk::U8(_),   DataType::U8)
        | (ColumnChunk::U16(_),  DataType::U16)
        | (ColumnChunk::U32(_),  DataType::U32)
        | (ColumnChunk::U64(_),  DataType::U64)
        | (ColumnChunk::F32(_),  DataType::F32)
        | (ColumnChunk::F64(_),  DataType::F64)
        | (ColumnChunk::Bool(_), DataType::Bool)
        | (ColumnChunk::Str(_),  DataType::Str)
    );
    if !ok {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            format!(
                "column '{}': chunk type does not match schema type {:?}",
                col.name, col.data_type
            ),
        ));
    }
    Ok(())
}

fn write_one_column(
    part_dir: &Path,
    col: &ColumnDef,
    chunk: &ColumnChunk,
) -> io::Result<ColumnStats> {
    match chunk {
        ColumnChunk::I8(v)   => write_numeric::<i8>(part_dir, &col.name, v),
        ColumnChunk::I16(v)  => write_numeric::<i16>(part_dir, &col.name, v),
        ColumnChunk::I32(v)  => write_numeric::<i32>(part_dir, &col.name, v),
        ColumnChunk::I64(v)  => write_numeric::<i64>(part_dir, &col.name, v),
        ColumnChunk::U8(v)   => write_numeric::<u8>(part_dir, &col.name, v),
        ColumnChunk::U16(v)  => write_numeric::<u16>(part_dir, &col.name, v),
        ColumnChunk::U32(v)  => write_numeric::<u32>(part_dir, &col.name, v),
        ColumnChunk::U64(v)  => write_numeric::<u64>(part_dir, &col.name, v),
        ColumnChunk::F32(v)  => write_numeric::<f32>(part_dir, &col.name, v),
        ColumnChunk::F64(v)  => write_numeric::<f64>(part_dir, &col.name, v),
        ColumnChunk::Bool(v) => write_numeric::<bool>(part_dir, &col.name, v),
        ColumnChunk::Str(v) => {
            let mut w = StringColumnWriter::create(part_dir, &col.name)?;
            w.write_chunk(v)?;
            w.finish()
        }
    }
}

fn write_numeric<T: IDataType>(
    part_dir: &Path,
    col_name: &str,
    values: &[T],
) -> io::Result<ColumnStats> {
    let mut w = ColumnWriter::<T>::create(part_dir, col_name)?;
    w.write_chunk(values)?;
    w.finish()
}

fn scan_next_part_id(table_dir: &Path) -> io::Result<u32> {
    let mut max_id: i64 = -1;
    for entry in fs::read_dir(table_dir)? {
        let entry = entry?;
        let name = entry.file_name();
        let name = name.to_string_lossy();
        if let Some(rest) = name.strip_prefix("part_") {
            if let Ok(id) = rest.parse::<u32>() {
                max_id = max_id.max(id as i64);
            }
        }
    }
    Ok((max_id + 1) as u32)
}
