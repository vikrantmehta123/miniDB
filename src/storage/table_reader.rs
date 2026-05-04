use std::io;
use std::path::{Path, PathBuf};

use crate::storage::column_chunk::ColumnChunk;
use crate::storage::column_reader::ColumnReader;
use crate::storage::part_discovery::discover_parts;
use crate::storage::schema::{DataType, TableDef};
use crate::storage::string_column_reader::StringColumnReader;

pub struct TableReader {
    table_dir: PathBuf,
    schema: TableDef,
}

impl TableReader {
    pub fn open(table_dir: &Path) -> io::Result<Self> {
        let schema = TableDef::open(table_dir)?;
        Ok(Self {
            table_dir: table_dir.to_path_buf(),
            schema,
        })
    }

    pub fn read_all(
        &self,
        columns: &[crate::storage::schema::ColumnDef],
    ) -> io::Result<Vec<ColumnChunk>> {
        let part_ids = discover_parts(&self.table_dir)?;
        let mut acc: Vec<Option<ColumnChunk>> = (0..columns.len()).map(|_| None).collect();

        for part_id in part_ids {
            let part_dir = TableDef::part_dir(&self.table_dir, part_id);
            for (i, col) in columns.iter().enumerate() {
                let chunk = match col.data_type {
                    DataType::I8 => {
                        ColumnChunk::I8(ColumnReader::open(&part_dir, &col.name)?.read_all()?)
                    }
                    DataType::I16 => {
                        ColumnChunk::I16(ColumnReader::open(&part_dir, &col.name)?.read_all()?)
                    }
                    DataType::I32 => {
                        ColumnChunk::I32(ColumnReader::open(&part_dir, &col.name)?.read_all()?)
                    }
                    DataType::I64 => {
                        ColumnChunk::I64(ColumnReader::open(&part_dir, &col.name)?.read_all()?)
                    }
                    DataType::U8 => {
                        ColumnChunk::U8(ColumnReader::open(&part_dir, &col.name)?.read_all()?)
                    }
                    DataType::U16 => {
                        ColumnChunk::U16(ColumnReader::open(&part_dir, &col.name)?.read_all()?)
                    }
                    DataType::U32 => {
                        ColumnChunk::U32(ColumnReader::open(&part_dir, &col.name)?.read_all()?)
                    }
                    DataType::U64 => {
                        ColumnChunk::U64(ColumnReader::open(&part_dir, &col.name)?.read_all()?)
                    }
                    DataType::F32 => {
                        ColumnChunk::F32(ColumnReader::open(&part_dir, &col.name)?.read_all()?)
                    }
                    DataType::F64 => {
                        ColumnChunk::F64(ColumnReader::open(&part_dir, &col.name)?.read_all()?)
                    }
                    DataType::Bool => {
                        ColumnChunk::Bool(ColumnReader::open(&part_dir, &col.name)?.read_all()?)
                    }
                    DataType::Str => ColumnChunk::Str(
                        StringColumnReader::open(&part_dir, &col.name)?.read_all()?,
                    ),
                };
                match &mut acc[i] {
                    None => acc[i] = Some(chunk),
                    Some(existing) => extend_chunk(existing, chunk),
                }
            }
        }

        Ok(acc.into_iter().flatten().collect())
    }
}

fn extend_chunk(dst: &mut ColumnChunk, src: ColumnChunk) {
    match (dst, src) {
        (ColumnChunk::I8(d), ColumnChunk::I8(s)) => d.extend(s),
        (ColumnChunk::I16(d), ColumnChunk::I16(s)) => d.extend(s),
        (ColumnChunk::I32(d), ColumnChunk::I32(s)) => d.extend(s),
        (ColumnChunk::I64(d), ColumnChunk::I64(s)) => d.extend(s),
        (ColumnChunk::U8(d), ColumnChunk::U8(s)) => d.extend(s),
        (ColumnChunk::U16(d), ColumnChunk::U16(s)) => d.extend(s),
        (ColumnChunk::U32(d), ColumnChunk::U32(s)) => d.extend(s),
        (ColumnChunk::U64(d), ColumnChunk::U64(s)) => d.extend(s),
        (ColumnChunk::F32(d), ColumnChunk::F32(s)) => d.extend(s),
        (ColumnChunk::F64(d), ColumnChunk::F64(s)) => d.extend(s),
        (ColumnChunk::Bool(d), ColumnChunk::Bool(s)) => d.extend(s),
        (ColumnChunk::Str(d), ColumnChunk::Str(s)) => d.extend(s),
        _ => {}
    }
}
