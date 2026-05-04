use std::io;
use std::path::Path;

use crate::storage::column_reader::ColumnReader;
use crate::storage::schema::{DataType, TableDef};
use crate::storage::column_chunk::{ColumnChunk};
use crate::storage::string_column_reader::StringColumnReader;

enum AnyReader {
    Numeric(ColumnReader),
    String(StringColumnReader),
}

pub struct TableReader {
    schema: TableDef,
    readers: Vec<AnyReader>,
}

impl TableReader {
    pub fn open(table_dir: &Path, part_id: u32) -> io::Result<Self> {
        let schema = TableDef::open(table_dir)?;
        let part_dir = TableDef::part_dir(table_dir, part_id);

        let readers = schema
            .columns
            .iter()
            .map(|col| -> io::Result<AnyReader> {
                Ok(match col.data_type {
                    DataType::Str => {
                        AnyReader::String(StringColumnReader::open(&part_dir, &col.name)?)
                    }
                    _ => AnyReader::Numeric(ColumnReader::open(&part_dir, &col.name)?),
                })
            })
            .collect::<io::Result<Vec<_>>>()?;

        Ok(Self { schema, readers })
    }

    pub fn granule_count(&self) -> usize {
        match self.readers.first() {
            Some(AnyReader::Numeric(r)) => r.granule_count(),
            Some(AnyReader::String(r)) => r.granule_count(),
            None => 0,
        }
    }

    /// Read the i-th granule across all columns, in schema order.
    pub fn read_granule(&mut self, idx: usize) -> io::Result<Vec<ColumnChunk>> {
        self.schema
            .columns
            .iter()
            .zip(self.readers.iter_mut())
            .map(|(col, reader)| match (&col.data_type, reader) {
                (DataType::I8,   AnyReader::Numeric(r)) => r.read_granule::<i8>(idx).map(ColumnChunk::I8),
                (DataType::I16,  AnyReader::Numeric(r)) => r.read_granule::<i16>(idx).map(ColumnChunk::I16),
                (DataType::I32,  AnyReader::Numeric(r)) => r.read_granule::<i32>(idx).map(ColumnChunk::I32),
                (DataType::I64,  AnyReader::Numeric(r)) => r.read_granule::<i64>(idx).map(ColumnChunk::I64),
                (DataType::U8,   AnyReader::Numeric(r)) => r.read_granule::<u8>(idx).map(ColumnChunk::U8),
                (DataType::U16,  AnyReader::Numeric(r)) => r.read_granule::<u16>(idx).map(ColumnChunk::U16),
                (DataType::U32,  AnyReader::Numeric(r)) => r.read_granule::<u32>(idx).map(ColumnChunk::U32),
                (DataType::U64,  AnyReader::Numeric(r)) => r.read_granule::<u64>(idx).map(ColumnChunk::U64),
                (DataType::F32,  AnyReader::Numeric(r)) => r.read_granule::<f32>(idx).map(ColumnChunk::F32),
                (DataType::F64,  AnyReader::Numeric(r)) => r.read_granule::<f64>(idx).map(ColumnChunk::F64),
                (DataType::Bool, AnyReader::Numeric(r)) => r.read_granule::<bool>(idx).map(ColumnChunk::Bool),
                (DataType::Str,  AnyReader::String(r))  => r.read_granule(idx).map(ColumnChunk::Str),
                _ => unreachable!("reader/dtype mismatch — bug in TableReader::open"),
            })
            .collect()
    }
}
