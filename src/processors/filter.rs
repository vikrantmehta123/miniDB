use crate::evaluator::evaluate;
use crate::parser::ast::Predicate;

use super::{
    batch::Batch,
    processor::{ExecutionError, Processor},
};

pub struct Filter {
    input: Box<dyn Processor>,
    predicate: Predicate,
}

impl Filter {
    pub fn new(input: Box<dyn Processor>, predicate: Predicate) -> Self {
        Self { input, predicate }
    }
}

impl Processor for Filter {
    fn next_batch(&mut self) -> Option<Result<Batch, ExecutionError>> {
        let batch = match self.input.next_batch()? {
            Ok(b) => b,
            Err(e) => return Some(Err(e)),
        };

        let named: Vec<(&str, &_)> = batch
            .schema
            .iter()
            .map(|c| c.name.as_str())
            .zip(batch.columns.iter())
            .collect();

        let mask = evaluate(&self.predicate, &named)
            .map_err(|e| ExecutionError::InvalidData(e.to_string()));

        match mask {
            Ok(m) => Some(Ok(Batch { selection: Some(m), ..batch })),
            Err(e) => Some(Err(e)),
        }
    }
}
