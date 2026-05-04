use crate::parser::ast::{CmpOp, Literal, Predicate};
use crate::storage::column_chunk::ColumnChunk;

#[derive(Debug)]
pub enum EvalError {
    ColumnNotFound(String),
    TypeMismatch(String),
}

impl std::fmt::Display for EvalError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            EvalError::ColumnNotFound(c) => write!(f, "column not found: {c}"),
            EvalError::TypeMismatch(msg) => write!(f, "type mismatch: {msg}"),
        }
    }
}

pub fn evaluate(
    predicate: &Predicate,
    columns: &[(&str, &ColumnChunk)],
) -> Result<Vec<bool>, EvalError> {
    match predicate {
        Predicate::Cmp { col, op, value } => {
            let chunk = columns
                .iter()
                .find(|(name, _)| *name == col.as_str())
                .map(|(_, c)| *c)
                .ok_or_else(|| EvalError::ColumnNotFound(col.clone()))?;
            eval_cmp(chunk, op, value, col)
        }
        Predicate::And(a, b) => {
            let ma = evaluate(a, columns)?;
            let mb = evaluate(b, columns)?;
            Ok(ma.iter().zip(mb.iter()).map(|(x, y)| x & y).collect())
        }
        Predicate::Or(a, b) => {
            let ma = evaluate(a, columns)?;
            let mb = evaluate(b, columns)?;
            Ok(ma.iter().zip(mb.iter()).map(|(x, y)| x | y).collect())
        }
        Predicate::Not(inner) => {
            let m = evaluate(inner, columns)?;
            Ok(m.iter().map(|x| !x).collect())
        }
    }
}

fn apply_op<T: PartialOrd>(val: T, op: &CmpOp, rhs: T) -> bool {
    match op {
        CmpOp::Eq => val == rhs,
        CmpOp::Ne => val != rhs,
        CmpOp::Lt => val < rhs,
        CmpOp::Le => val <= rhs,
        CmpOp::Gt => val > rhs,
        CmpOp::Ge => val >= rhs,
    }
}

fn eval_cmp(
    chunk: &ColumnChunk,
    op: &CmpOp,
    value: &Literal,
    col: &str,
) -> Result<Vec<bool>, EvalError> {
    match (chunk, value) {
        (ColumnChunk::I8(v),  Literal::Int(n))   => Ok(v.iter().map(|&x| apply_op(x as i64, op, *n)).collect()),
        (ColumnChunk::I16(v), Literal::Int(n))   => Ok(v.iter().map(|&x| apply_op(x as i64, op, *n)).collect()),
        (ColumnChunk::I32(v), Literal::Int(n))   => Ok(v.iter().map(|&x| apply_op(x as i64, op, *n)).collect()),
        (ColumnChunk::I64(v), Literal::Int(n))   => Ok(v.iter().map(|&x| apply_op(x, op, *n)).collect()),
        (ColumnChunk::U8(v),  Literal::UInt(n))  => Ok(v.iter().map(|&x| apply_op(x as u64, op, *n)).collect()),
        (ColumnChunk::U16(v), Literal::UInt(n))  => Ok(v.iter().map(|&x| apply_op(x as u64, op, *n)).collect()),
        (ColumnChunk::U32(v), Literal::UInt(n))  => Ok(v.iter().map(|&x| apply_op(x as u64, op, *n)).collect()),
        (ColumnChunk::U64(v), Literal::UInt(n))  => Ok(v.iter().map(|&x| apply_op(x, op, *n)).collect()),
        (ColumnChunk::U8(v),  Literal::Int(n))   => Ok(v.iter().map(|&x| apply_op(x as i64, op, *n)).collect()),
        (ColumnChunk::U16(v), Literal::Int(n))   => Ok(v.iter().map(|&x| apply_op(x as i64, op, *n)).collect()),
        (ColumnChunk::U32(v), Literal::Int(n))   => Ok(v.iter().map(|&x| apply_op(x as i64, op, *n)).collect()),
        (ColumnChunk::U64(v), Literal::Int(n))   => Ok(v.iter().map(|&x| apply_op(x as i64, op, *n)).collect()),
        (ColumnChunk::F32(v), Literal::Float(n)) => Ok(v.iter().map(|&x| apply_op(x as f64, op, *n)).collect()),
        (ColumnChunk::F64(v), Literal::Float(n)) => Ok(v.iter().map(|&x| apply_op(x, op, *n)).collect()),
        (ColumnChunk::F32(v), Literal::Int(n))   => Ok(v.iter().map(|&x| apply_op(x as f64, op, *n as f64)).collect()),
        (ColumnChunk::F64(v), Literal::Int(n))   => Ok(v.iter().map(|&x| apply_op(x, op, *n as f64)).collect()),
        (ColumnChunk::Bool(v), Literal::Bool(b)) => Ok(v.iter().map(|&x| apply_op(x, op, *b)).collect()),
        (ColumnChunk::Str(v), Literal::Str(s))   => Ok(v.iter().map(|x| apply_op(x.as_str(), op, s.as_str())).collect()),
        _ => Err(EvalError::TypeMismatch(format!(
            "column '{col}' type does not match literal"
        ))),
    }
}
