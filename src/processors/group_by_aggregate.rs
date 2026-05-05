use std::collections::HashMap;

use crate::aggregator::{self, Aggregator};
use crate::parser::ast::AggFunc;
use crate::storage::column_chunk::ColumnChunk;
use crate::storage::schema::{ColumnDef, DataType};

use super::batch::Batch;
use super::processor::{ExecutionError, Processor};
use super::scalar_value::{GroupKey, ScalarValue};

/// Describes one aggregate expression in the SELECT list.
pub struct AggSpec {
    pub func: AggFunc,
    /// Index of the input column in the batch from the child processor.
    pub input_col_idx: usize,
    /// Needed to instantiate fresh aggregators when a new group key appears.
    pub input_type: DataType,
    pub output_col: ColumnDef,
}

pub struct GroupByAggregate {
    input: Box<dyn Processor>,
    /// Indices of the GROUP BY columns in the input batch.
    group_by_indices: Vec<usize>,
    /// Schema of the GROUP BY columns, carried into the output batch.
    group_by_schema: Vec<ColumnDef>,
    agg_specs: Vec<AggSpec>,
    /// One aggregator-set per distinct group key.
    groups: HashMap<GroupKey, Vec<Box<dyn Aggregator>>>,
    done: bool,
}

impl GroupByAggregate {
    pub fn new(
        input: Box<dyn Processor>,
        group_by_indices: Vec<usize>,
        group_by_schema: Vec<ColumnDef>,
        agg_specs: Vec<AggSpec>,
    ) -> Self {
        Self {
            input,
            group_by_indices,
            group_by_schema,
            agg_specs,
            groups: HashMap::new(),
            done: false,
        }
    }

    /// Route every row in `batch` to its group and update its aggregators.
    ///
    /// Takes fields as separate parameters because we need `groups` mutably
    /// and `agg_specs`/`group_by_indices` as shared refs simultaneously —
    /// the borrow checker cannot prove disjointness through `&mut self`.
    fn drain_batch(
        groups: &mut HashMap<GroupKey, Vec<Box<dyn Aggregator>>>,
        agg_specs: &[AggSpec],
        group_by_indices: &[usize],
        batch: &Batch,
    ) -> Result<(), ExecutionError> {
        let n_rows = batch.columns[0].len();

        for row in 0..n_rows {
            let key: GroupKey = group_by_indices
                .iter()
                .map(|&idx| ScalarValue::from_chunk(&batch.columns[idx], row))
                .collect();

            // Look up or create the aggregator set for this group key.
            // Using Occupied/Vacant directly avoids a second HashMap lookup
            // and keeps the borrow on `groups` separate from `agg_specs`.
            let aggs = match groups.entry(key) {
                std::collections::hash_map::Entry::Occupied(e) => e.into_mut(),
                std::collections::hash_map::Entry::Vacant(e) => {
                    let fresh: Vec<Box<dyn Aggregator>> = agg_specs
                        .iter()
                        .map(|spec| {
                            // Types were validated by the analyser — this should
                            // never fail at runtime.
                            aggregator::build(spec.func.clone(), spec.input_type.clone())
                                .expect("aggregator build failed: type should have been validated")
                        })
                        .collect();
                    e.insert(fresh)
                }
            };

            // Feed a one-element chunk for this row to each aggregator.
            for (agg, spec) in aggs.iter_mut().zip(agg_specs.iter()) {
                let single_row = ScalarValue::build_column(vec![
                    ScalarValue::from_chunk(&batch.columns[spec.input_col_idx], row),
                ]);
                agg.update(&single_row)?;
            }
        }

        Ok(())
    }

    /// Finalize all groups and build the output Batch.
    fn finalize_groups(
        group_by_schema: &[ColumnDef],
        agg_specs: &[AggSpec],
        groups: &mut HashMap<GroupKey, Vec<Box<dyn Aggregator>>>,
    ) -> Batch {
        let n_group_by = group_by_schema.len();
        let n_aggs = agg_specs.len();
        let n_groups = groups.len();

        // Collect one ScalarValue per (group, column) position, then
        // convert to ColumnChunks column-by-column at the end.
        let mut key_cols: Vec<Vec<ScalarValue>> =
            (0..n_group_by).map(|_| Vec::with_capacity(n_groups)).collect();
        let mut agg_scalars: Vec<Vec<ScalarValue>> =
            (0..n_aggs).map(|_| Vec::with_capacity(n_groups)).collect();

        for (key, mut aggs) in groups.drain() {
            for (i, scalar) in key.into_iter().enumerate() {
                key_cols[i].push(scalar);
            }
            for (i, agg) in aggs.iter_mut().enumerate() {
                // finalize() produces a single-row ColumnChunk; we extract row 0.
                let chunk = agg.finalize();
                agg_scalars[i].push(ScalarValue::from_chunk(&chunk, 0));
            }
        }

        let mut schema: Vec<ColumnDef> = group_by_schema.to_vec();
        schema.extend(agg_specs.iter().map(|s| s.output_col.clone()));

        let mut columns: Vec<ColumnChunk> =
            key_cols.into_iter().map(ScalarValue::build_column).collect();
        columns.extend(agg_scalars.into_iter().map(ScalarValue::build_column));

        Batch { schema, columns }
    }
}

impl Processor for GroupByAggregate {
    fn next_batch(&mut self) -> Option<Result<Batch, ExecutionError>> {
        if self.done {
            return None;
        }

        // Drain: consume all input and accumulate group state.
        while let Some(result) = self.input.next_batch() {
            let batch = match result {
                Ok(b) => b,
                Err(e) => return Some(Err(e)),
            };
            if let Err(e) = Self::drain_batch(
                &mut self.groups,
                &self.agg_specs,
                &self.group_by_indices,
                &batch,
            ) {
                return Some(Err(e));
            }
        }

        self.done = true;
        Some(Ok(Self::finalize_groups(
            &self.group_by_schema,
            &self.agg_specs,
            &mut self.groups,
        )))
    }
}
