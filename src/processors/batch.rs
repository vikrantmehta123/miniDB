use crate::storage::{column_chunk::ColumnChunk, schema::ColumnDef};

pub struct Batch {
    pub schema: Vec<ColumnDef>,
    pub columns: Vec<ColumnChunk>,
}
