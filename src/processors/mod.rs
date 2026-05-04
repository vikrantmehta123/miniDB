pub mod batch;
pub mod filter;
pub mod full_scan;
pub mod processor;
pub mod projection;

use std::path::PathBuf;

use crate::parser::ast::{Predicate, Projection as AstProjection, SelectStmt};
use crate::storage::schema::{ColumnDef, TableDef};

use self::{
    filter::Filter,
    full_scan::FullScan,
    processor::{ExecutionError, Processor},
    projection::Projection,
};

pub fn build_plan(
    table_dir: PathBuf,
    stmt: &SelectStmt,
    schema: &TableDef,
) -> Result<Box<dyn Processor>, ExecutionError> {
    let proj_names: Vec<&str> = match &stmt.projection {
        AstProjection::All => schema.columns.iter().map(|c| c.name.as_str()).collect(),
        AstProjection::Columns(names) => names.iter().map(|s| s.as_str()).collect(),
    };

    let mut pred_names: Vec<&str> = Vec::new();
    if let Some(pred) = &stmt.where_clause {
        collect_pred_cols(pred, &mut pred_names);
    }

    let scan_cols: Vec<ColumnDef> = schema
        .columns
        .iter()
        .filter(|c| proj_names.contains(&c.name.as_str()) || pred_names.contains(&c.name.as_str()))
        .cloned()
        .collect();

    let mut node: Box<dyn Processor> = Box::new(FullScan::new(table_dir, scan_cols)?);

    if let Some(pred) = stmt.where_clause.clone() {
        node = Box::new(Filter::new(node, pred));
    }

    let output_names: Vec<String> = match &stmt.projection {
        AstProjection::All => schema.columns.iter().map(|c| c.name.clone()).collect(),
        AstProjection::Columns(names) => names.clone(),
    };
    node = Box::new(Projection::new(node, output_names));

    Ok(node)
}

fn collect_pred_cols<'a>(pred: &'a Predicate, out: &mut Vec<&'a str>) {
    match pred {
        Predicate::Cmp { col, .. } => out.push(col.as_str()),
        Predicate::And(a, b) | Predicate::Or(a, b) => {
            collect_pred_cols(a, out);
            collect_pred_cols(b, out);
        }
        Predicate::Not(inner) => collect_pred_cols(inner, out),
    }
}
