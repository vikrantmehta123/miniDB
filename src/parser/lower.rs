use sqlparser::ast as s;
use crate::parser::{ParseError, Statement, InsertStmt, Literal};
use crate::parser::ast::{SelectStmt, Projection};

pub fn lower(stmt: s::Statement) -> Result<Statement, ParseError> {
    match stmt {
        s::Statement::Insert(ins) => lower_insert(ins).map(Statement::Insert),
        s::Statement::Query(q) => lower_select(*q).map(Statement::Select),
        other => Err(ParseError::Unsupported(format!("statement: {:?}", other))),
    }
}


fn lower_insert(ins: s::Insert) -> Result<InsertStmt, ParseError> {
    let table = ins.table.to_string();
    let columns = if ins.columns.is_empty() {
        None
    } else {
        Some(ins.columns.into_iter().map(|c| c.value).collect())
    };
    let source = ins.source
        .ok_or_else(|| ParseError::Unsupported("INSERT without VALUES".into()))?;
    let rows = match *source.body {
        s::SetExpr::Values(s::Values { rows, .. }) => rows
            .into_iter()
            .map(|row| row.into_iter().map(lower_expr).collect())
            .collect::<Result<Vec<_>, _>>()?,
        _ => return Err(ParseError::Unsupported("INSERT must use VALUES".into())),
    };
    Ok(InsertStmt { table, columns, rows })
}

fn lower_expr(e: s::Expr) -> Result<Literal, ParseError> {
    match e {
        s::Expr::Value(v) => lower_value(v.value),
        s::Expr::UnaryOp { op: s::UnaryOperator::Minus, expr } => match lower_expr(*expr)? {
            Literal::Int(i)    => Ok(Literal::Int(-i)),
            Literal::Float(f)  => Ok(Literal::Float(-f)),
            Literal::UInt(u)   => Ok(Literal::Int(-(u as i64))),  // narrow path
            _ => Err(ParseError::Unsupported("unary minus on non-numeric".into())),
        },
        other => Err(ParseError::Unsupported(format!("expr in VALUES: {:?}", other))),
    }
}

fn lower_value(v: s::Value) -> Result<Literal, ParseError> {
    match v {
        s::Value::Number(n, _) => {
            if let Ok(i) = n.parse::<i64>()      { Ok(Literal::Int(i)) }
            else if let Ok(u) = n.parse::<u64>() { Ok(Literal::UInt(u)) }
            else if let Ok(f) = n.parse::<f64>() { Ok(Literal::Float(f)) }
            else { Err(ParseError::Syntax(format!("bad number: {n}"))) }
        }
        s::Value::SingleQuotedString(s) => Ok(Literal::Str(s)),
        s::Value::Boolean(b)            => Ok(Literal::Bool(b)),
        s::Value::Null                  => Ok(Literal::Null),
        other => Err(ParseError::Unsupported(format!("literal: {:?}", other))),
    }
}

fn lower_select(query: s::Query) -> Result<SelectStmt, ParseError> {
    let select = match *query.body {
        s::SetExpr::Select(s) => s,
        _ => return Err(ParseError::Unsupported("only plain SELECT supported".into())),
    };
    let table = select.from.into_iter().next()
        .ok_or_else(|| ParseError::Unsupported("SELECT requires a FROM clause".into()))
        .and_then(|t| match t.relation {
            s::TableFactor::Table { name, .. } => Ok(name.to_string()),
            _ => Err(ParseError::Unsupported("only simple table references supported".into())),
        })?;
    let projection = if select.projection.len() == 1 {
        if let s::SelectItem::Wildcard(_) = &select.projection[0] {
            Projection::All
        } else {
            lower_projection(select.projection)?
        }
    } else {
        lower_projection(select.projection)?
    };
    Ok(SelectStmt { table, projection })
}

fn lower_projection(items: Vec<s::SelectItem>) -> Result<Projection, ParseError> {
    let cols = items.into_iter().map(|item| match item {
        s::SelectItem::UnnamedExpr(s::Expr::Identifier(id)) => Ok(id.value),
        other => Err(ParseError::Unsupported(format!("unsupported projection item: {:?}", other))),
    }).collect::<Result<Vec<_>, _>>()?;
    Ok(Projection::Columns(cols))
}
