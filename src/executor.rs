pub use crate::analyser::{InsertError, SelectError, ScanPlan};
use crate::analyser::{analyse_insert, analyse_select};
use crate::parser::InsertStmt;
use crate::parser::ast::SelectStmt;
use crate::parser::Literal;
use crate::storage::column_chunk::ColumnChunk;
use crate::storage::schema::{DataType, TableDef};
use crate::storage::table_writer::{PartMetadata, TableWriter};
use std::path::PathBuf;

pub fn execute_insert(
    stmt: InsertStmt,
    schema: &TableDef,
    table_dir: PathBuf,
) -> Result<PartMetadata, InsertError> {
    analyse_insert(&stmt, schema)?;
    let chunks = sort_and_transpose(stmt, schema);
    let writer = TableWriter::open(table_dir)?;
    let meta = writer.insert(chunks)?;
    Ok(meta)
}

pub fn execute_select(
    stmt: SelectStmt,
    schema: &TableDef,
    table_dir: PathBuf,
) -> Result<Vec<ColumnChunk>, SelectError> {
    let plan = analyse_select(&stmt, schema, table_dir)?;
    let reader = crate::storage::table_reader::TableReader::open(&plan.table_dir)?;
    let chunks = reader.read_all(&plan.columns)?;
    Ok(chunks)
}

fn sort_and_transpose(stmt: InsertStmt, schema: &TableDef) -> Vec<ColumnChunk> {
    let mut rows = stmt.rows;

    rows.sort_by(|a, b| {
        for &key_col in &schema.sort_key {
            let ord = compare_literals(&a[key_col], &b[key_col]);
            if ord != std::cmp::Ordering::Equal {
                return ord;
            }
        }
        std::cmp::Ordering::Equal
    });

    let n_cols = schema.columns.len();
    let mut chunks: Vec<ColumnChunk> = schema
        .columns
        .iter()
        .map(|col| match col.data_type {
            DataType::I8 => ColumnChunk::I8(Vec::new()),
            DataType::I16 => ColumnChunk::I16(Vec::new()),
            DataType::I32 => ColumnChunk::I32(Vec::new()),
            DataType::I64 => ColumnChunk::I64(Vec::new()),
            DataType::U8 => ColumnChunk::U8(Vec::new()),
            DataType::U16 => ColumnChunk::U16(Vec::new()),
            DataType::U32 => ColumnChunk::U32(Vec::new()),
            DataType::U64 => ColumnChunk::U64(Vec::new()),
            DataType::F32 => ColumnChunk::F32(Vec::new()),
            DataType::F64 => ColumnChunk::F64(Vec::new()),
            DataType::Bool => ColumnChunk::Bool(Vec::new()),
            DataType::Str => ColumnChunk::Str(Vec::new()),
        })
        .collect();

    for row in rows {
        for (col_idx, lit) in row.into_iter().enumerate() {
            push_literal(&mut chunks[col_idx], lit);
        }
    }

    chunks
}

fn compare_literals(a: &Literal, b: &Literal) -> std::cmp::Ordering {
    match (a, b) {
        (Literal::Int(x), Literal::Int(y)) => x.cmp(y),
        (Literal::UInt(x), Literal::UInt(y)) => x.cmp(y),
        (Literal::Float(x), Literal::Float(y)) => {
            x.partial_cmp(y).unwrap_or(std::cmp::Ordering::Equal)
        }
        (Literal::Str(x), Literal::Str(y)) => x.cmp(y),
        (Literal::Bool(x), Literal::Bool(y)) => x.cmp(y),
        _ => std::cmp::Ordering::Equal,
    }
}

fn push_literal(chunk: &mut ColumnChunk, lit: Literal) {
    match (chunk, lit) {
        (ColumnChunk::I8(v), Literal::Int(i)) => v.push(i as i8),
        (ColumnChunk::I16(v), Literal::Int(i)) => v.push(i as i16),
        (ColumnChunk::I32(v), Literal::Int(i)) => v.push(i as i32),
        (ColumnChunk::I64(v), Literal::Int(i)) => v.push(i as i64),
        (ColumnChunk::U8(v), Literal::UInt(u)) => v.push(u as u8),
        (ColumnChunk::U16(v), Literal::UInt(u)) => v.push(u as u16),
        (ColumnChunk::U32(v), Literal::UInt(u)) => v.push(u as u32),
        (ColumnChunk::U64(v), Literal::UInt(u)) => v.push(u as u64),
        (ColumnChunk::U8(v), Literal::Int(i)) => v.push(i as u8),
        (ColumnChunk::U16(v), Literal::Int(i)) => v.push(i as u16),
        (ColumnChunk::U32(v), Literal::Int(i)) => v.push(i as u32),
        (ColumnChunk::U64(v), Literal::Int(i)) => v.push(i as u64),
        (ColumnChunk::F32(v), Literal::Float(f)) => v.push(f as f32),
        (ColumnChunk::F64(v), Literal::Float(f)) => v.push(f as f64),
        (ColumnChunk::F32(v), Literal::Int(i)) => v.push(i as f32),
        (ColumnChunk::F64(v), Literal::Int(i)) => v.push(i as f64),
        (ColumnChunk::Bool(v), Literal::Bool(b)) => v.push(b),
        (ColumnChunk::Str(v), Literal::Str(s)) => v.push(s),
        _ => {}
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::storage::schema::{ColumnDef, DataType, TableDef};
    use std::path::PathBuf;

    fn make_schema() -> TableDef {
        TableDef {
            name: "events".into(),
            columns: vec![
                ColumnDef {
                    name: "ts".into(),
                    data_type: DataType::I64,
                },
                ColumnDef {
                    name: "uid".into(),
                    data_type: DataType::U32,
                },
                ColumnDef {
                    name: "tag".into(),
                    data_type: DataType::Str,
                },
            ],
            sort_key: vec![0],
        }
    }

    #[test]
    fn insert_creates_part_on_disk() {
        let dir = std::env::temp_dir().join("tinyolap_executor_test");
        let _ = std::fs::remove_dir_all(&dir);
        TableDef::create(&dir, &make_schema()).unwrap();

        let sql = "INSERT INTO events VALUES (2, 20, 'b'), (1, 10, 'a'), (3, 30, 'c')";
        let stmt = crate::parser::parse(sql).unwrap();
        let schema = make_schema();

        let crate::parser::Statement::Insert(s) = stmt else {
            panic!("expected Insert")
        };
        let meta = execute_insert(s, &schema, dir.clone()).unwrap();

        assert_eq!(meta.rows, 3);
        let part_dir = dir.join(format!("part_{:05}", meta.part_id));
        assert!(part_dir.join("ts.bin").exists());
        assert!(part_dir.join("uid.bin").exists());
        assert!(part_dir.join("tag.bin").exists());
        assert!(part_dir.join("ts.bin").metadata().unwrap().len() > 0);

        std::fs::remove_dir_all(&dir).unwrap();
    }
}
