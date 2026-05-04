use crate::parser::{InsertStmt, Literal};
use crate::parser::ast::{Predicate, Projection, SelectStmt};
use crate::storage::schema::{DataType, TableDef};
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

#[derive(Debug)]
pub enum SelectError {
    UnknownTable(String),
    UnknownColumn(String),
    Io(std::io::Error),
    EvalError(crate::evaluator::EvalError),
}

impl std::fmt::Display for SelectError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SelectError::UnknownTable(t) => write!(f, "unknown table '{t}'"),
            SelectError::UnknownColumn(c) => write!(f, "unknown column '{c}'"),
            SelectError::Io(e) => write!(f, "I/O error: {e}"),
            SelectError::EvalError(e) => write!(f, "eval error: {e}"),
        }
    }
}

impl From<std::io::Error> for SelectError {
    fn from(e: std::io::Error) -> Self {
        SelectError::Io(e)
    }
}

impl From<crate::evaluator::EvalError> for SelectError {
    fn from(e: crate::evaluator::EvalError) -> Self {
        SelectError::EvalError(e)
    }
}


pub struct ScanPlan {
    pub table_dir: PathBuf,
    pub columns: Vec<crate::storage::schema::ColumnDef>,
}

pub fn analyse_insert(stmt: &InsertStmt, schema: &TableDef) -> Result<(), InsertError> {
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

fn literal_compatible(lit: &Literal, dt: &DataType) -> bool {
    match (lit, dt) {
        (Literal::Int(_), DataType::I8 | DataType::I16 | DataType::I32 | DataType::I64) => true,
        (Literal::Int(i), DataType::U8 | DataType::U16 | DataType::U32 | DataType::U64) => *i >= 0,
        (Literal::UInt(_), DataType::U8 | DataType::U16 | DataType::U32 | DataType::U64) => true,
        (Literal::Float(_), DataType::F32 | DataType::F64) => true,
        (Literal::Int(_) | Literal::UInt(_), DataType::F32 | DataType::F64) => true,
        (Literal::Bool(_), DataType::Bool) => true,
        (Literal::Str(_), DataType::Str) => true,
        _ => false,
    }
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
