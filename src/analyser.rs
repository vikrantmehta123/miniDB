use crate::parser::{InsertStmt, Literal};
use crate::parser::ast::{Predicate, Projection, SelectExpr, SelectStmt};
use crate::storage::schema::{DataType, TableDef};


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
    fn from(e: std::io::Error) -> Self {
        SelectError::Io(e)
    }
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
    mut stmt: SelectStmt,
    schema: &TableDef,
) -> Result<SelectStmt, SelectError> {
    if stmt.table != schema.name {
        return Err(SelectError::UnknownTable(stmt.table.clone()));
    }

    stmt.projection = match stmt.projection {
        Projection::All => {
            let exprs = schema.columns.iter()
                .map(|c| SelectExpr::Col(c.name.clone()))
                .collect();
            Projection::Exprs(exprs)
        }
        Projection::Exprs(ref exprs) => {
            for expr in exprs {
                match expr {
                    SelectExpr::Col(name) => {
                        if schema.columns.iter().all(|c| &c.name != name) {
                            return Err(SelectError::UnknownColumn(name.clone()));
                        }
                    }
                    SelectExpr::Agg { col, .. } if col != "*" => {
                        if schema.columns.iter().all(|c| &c.name != col) {
                            return Err(SelectError::UnknownColumn(col.clone()));
                        }
                    }
                    _ => {}
                }
            }
            stmt.projection
        }
    };

    if let Some(pred) = &stmt.where_clause {
        validate_predicate(pred, schema)?;
    }

    Ok(stmt)
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
