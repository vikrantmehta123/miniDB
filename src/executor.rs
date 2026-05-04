use crate::parser::{InsertStmt, Literal};
use crate::parser::ast::{Projection, Predicate, SelectStmt};
use crate::storage::column_chunk::ColumnChunk;
use crate::storage::schema::{DataType, TableDef};
use crate::storage::table_writer::{PartMetadata, TableWriter};
use std::path::PathBuf;

#[derive(Debug)]
pub enum InsertError {
    UnknownTable(String),
    ColumnCountMismatch {
        row: usize,
        expected: usize,
        got: usize,
    },
    TypeMismatch {
        row: usize,
        col: usize,
        col_name: String,
    },
    NullNotAllowed {
        row: usize,
        col: usize,
        col_name: String,
    },
    Io(std::io::Error),
}

impl std::fmt::Display for InsertError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            InsertError::UnknownTable(t) => write!(f, "unknown table '{t}'"),
            InsertError::ColumnCountMismatch { row, expected, got } => {
                write!(f, "row {row}: expected {expected} values, got {got}")
            }
            InsertError::TypeMismatch { row, col, col_name } => write!(
                f,
                "row {row}, col {col} ('{col_name}'): value type does not match column type"
            ),
            InsertError::NullNotAllowed { row, col, col_name } => write!(
                f,
                "row {row}, col {col} ('{col_name}'): NULL is not allowed"
            ),
            InsertError::Io(e) => write!(f, "I/O error: {e}"),
        }
    }
}

impl From<std::io::Error> for InsertError {
    fn from(e: std::io::Error) -> Self {
        InsertError::Io(e)
    }
}

pub fn analyse(stmt: &InsertStmt, schema: &TableDef) -> Result<(), InsertError> {
    if stmt.table != schema.name {
        return Err(InsertError::UnknownTable(stmt.table.clone()));
    }

    let expected = schema.columns.len();
    for (row_idx, row) in stmt.rows.iter().enumerate() {
        if row.len() != expected {
            return Err(InsertError::ColumnCountMismatch {
                row: row_idx,
                expected,
                got: row.len(),
            });
        }

        for (col_idx, lit) in row.iter().enumerate() {
            let col = &schema.columns[col_idx];
            if matches!(lit, Literal::Null) {
                return Err(InsertError::NullNotAllowed {
                    row: row_idx,
                    col: col_idx,
                    col_name: col.name.clone(),
                });
            }

            if !literal_compatible(lit, &col.data_type) {
                return Err(InsertError::TypeMismatch {
                    row: row_idx,
                    col: col_idx,
                    col_name: col.name.clone(),
                });
            }
        }
    }

    Ok(())
}

fn literal_compatible(lit: &Literal, dt: &DataType) -> bool {
    match (lit, dt) {
        (Literal::Int(_), DataType::I8 | DataType::I16 | DataType::I32 | DataType::I64) => true,
        (Literal::Int(i), DataType::U8 | DataType::U16 | DataType::U32 | DataType::U64) => *i >= 0,
        (Literal::UInt(_), DataType::U8 | DataType::U16 | DataType::U32 | DataType::U64) => true,
        (Literal::Float(_), DataType::F32 | DataType::F64) => true,
        (Literal::Int(_) | Literal::UInt(_), DataType::F32 | DataType::F64) => true, // integers coerce to float
        (Literal::Bool(_), DataType::Bool) => true,
        (Literal::Str(_), DataType::Str) => true,
        _ => false,
    }
}
pub fn sort_and_transpose(stmt: InsertStmt, schema: &TableDef) -> Vec<ColumnChunk> {
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

pub fn execute_insert(
    stmt: InsertStmt,
    schema: &TableDef,
    table_dir: PathBuf,
) -> Result<PartMetadata, InsertError> {
    analyse(&stmt, schema)?;
    let chunks = sort_and_transpose(stmt, schema);
    let writer = TableWriter::open(table_dir)?;
    let meta = writer.insert(chunks)?;
    Ok(meta)
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


// ==============================================================
// SELECT statement execution
// ==============================================================

#[derive(Debug)]
pub enum SelectError {
    UnknownTable(String),
    UnknownColumn(String),
    Io(std::io::Error),
}

impl std::fmt::Display for SelectError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SelectError::UnknownTable(t) => write!(f, "unknown table '{t}'"),
            SelectError::UnknownColumn(c) => write!(f, "unknown column '{c}'"),
            SelectError::Io(e) => write!(f, "I/O error: {e}"),
        }
    }
}

impl From<std::io::Error> for SelectError {
    fn from(e: std::io::Error) -> Self { SelectError::Io(e) }
}


pub struct ScanPlan {
    pub table_dir: PathBuf,
    pub columns: Vec<crate::storage::schema::ColumnDef>,
}

fn validate_predicate(pred: &Predicate, schema: &TableDef) -> Result<(), SelectError> {
    match pred {
        Predicate::Cmp { col, .. } => {
            if schema.columns.iter().any(|c| &c.name == col) {
                Ok(())
            } else {
                Err(SelectError::UnknownColumn(col.clone()))
            }
        }
        Predicate::And(l, r) | Predicate::Or(l, r) => {
            validate_predicate(l, schema)?;
            validate_predicate(r, schema)
        }
        Predicate::Not(inner) => validate_predicate(inner, schema),
    }
}


pub fn analyse_select(
    stmt: &SelectStmt,
    schema: &TableDef,
    table_dir: PathBuf,
) -> Result<ScanPlan, SelectError> {
    if stmt.table != schema.name {
        return Err(SelectError::UnknownTable(stmt.table.clone()));
    }
    let columns = match &stmt.projection {
        Projection::All => schema.columns.clone(),
        Projection::Columns(names) => {
            names.iter().map(|name| {
                schema.columns.iter()
                    .find(|col| &col.name == name)
                    .cloned()
                    .ok_or_else(|| SelectError::UnknownColumn(name.clone()))
            }).collect::<Result<Vec<_>, _>>()?
        }
    };
    if let Some(pred) = &stmt.where_clause {
        validate_predicate(pred, schema)?;
    }

    Ok(ScanPlan { table_dir, columns })
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
