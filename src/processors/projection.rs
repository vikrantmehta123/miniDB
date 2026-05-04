use super::{
    batch::Batch,
    processor::{ExecutionError, Processor},
};

pub struct Projection {
    input: Box<dyn Processor>,
    output_cols: Vec<String>,
}

impl Projection {
    pub fn new(input: Box<dyn Processor>, output_cols: Vec<String>) -> Self {
        Self { input, output_cols }
    }
}

impl Processor for Projection {
    fn next_batch(&mut self) -> Option<Result<Batch, ExecutionError>> {
        let batch = match self.input.next_batch()? {
            Ok(b) => b,
            Err(e) => return Some(Err(e)),
        };

        let schema_and_cols: Vec<_> = batch
            .schema
            .into_iter()
            .zip(batch.columns.into_iter())
            .filter(|(def, _)| self.output_cols.contains(&def.name))
            .collect();

        let (schema, columns): (Vec<_>, Vec<_>) = match &batch.selection {
            None => schema_and_cols.into_iter().unzip(),
            Some(mask) => schema_and_cols
                .into_iter()
                .map(|(def, chunk)| (def, chunk.filter(mask)))
                .unzip(),
        };

        Some(Ok(Batch { schema, columns, selection: None }))
    }
}
