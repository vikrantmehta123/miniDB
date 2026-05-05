use std::path::PathBuf;

use crate::storage::{
    column_chunk::ColumnChunk,
    column_reader::ColumnReader,
    part_discovery::discover_parts,
    schema::{ColumnDef, DataType, TableDef},
    string_column_reader::StringColumnReader,
};

use super::{
    batch::Batch,
    processor::{ExecutionError, Processor},
};

pub struct FullScan {
    table_dir: PathBuf,
    columns: Vec<ColumnDef>,
    part_ids: Vec<u32>,
    next_part: usize,
}

impl FullScan {
    pub fn new(table_dir: PathBuf, columns: Vec<ColumnDef>) -> Result<Self, ExecutionError> {
        let part_ids = discover_parts(&table_dir)?;
        Ok(Self { table_dir, columns, part_ids, next_part: 0 })
    }
}

impl Processor for FullScan {
    fn next_batch(&mut self) -> Option<Result<Batch, ExecutionError>> {
        if self.next_part >= self.part_ids.len() {
            return None;
        }
        let part_id = self.part_ids[self.next_part];
        self.next_part += 1;
        let part_dir = TableDef::part_dir(&self.table_dir, part_id);

        let result = (|| {
            let mut columns = Vec::with_capacity(self.columns.len());
            for col in &self.columns {
                let chunk = match col.data_type {
                    DataType::I8   => ColumnChunk::I8(ColumnReader::open(&part_dir, &col.name)?.read_all()?),
                    DataType::I16  => ColumnChunk::I16(ColumnReader::open(&part_dir, &col.name)?.read_all()?),
                    DataType::I32  => ColumnChunk::I32(ColumnReader::open(&part_dir, &col.name)?.read_all()?),
                    DataType::I64  => ColumnChunk::I64(ColumnReader::open(&part_dir, &col.name)?.read_all()?),
                    DataType::U8   => ColumnChunk::U8(ColumnReader::open(&part_dir, &col.name)?.read_all()?),
                    DataType::U16  => ColumnChunk::U16(ColumnReader::open(&part_dir, &col.name)?.read_all()?),
                    DataType::U32  => ColumnChunk::U32(ColumnReader::open(&part_dir, &col.name)?.read_all()?),
                    DataType::U64  => ColumnChunk::U64(ColumnReader::open(&part_dir, &col.name)?.read_all()?),
                    DataType::F32  => ColumnChunk::F32(ColumnReader::open(&part_dir, &col.name)?.read_all()?),
                    DataType::F64  => ColumnChunk::F64(ColumnReader::open(&part_dir, &col.name)?.read_all()?),
                    DataType::Bool => ColumnChunk::Bool(ColumnReader::open(&part_dir, &col.name)?.read_all()?),
                    DataType::Str  => ColumnChunk::Str(StringColumnReader::open(&part_dir, &col.name)?.read_all()?),
                };
                columns.push(chunk);
            }
            Ok(Batch { schema: self.columns.clone(), columns })
        })();

        Some(result)
    }
}
