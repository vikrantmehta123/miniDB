# TASK-006 — GROUP BY with Hash Aggregation

## Status: Completed

## What was built
- `group_by: Vec<String>` added to `SelectStmt` in `ast.rs`
- `lower_group_by` helper + validation in `lower_select`: every bare `Col` in the projection must appear in the GROUP BY list
- `ScalarValue` / `GroupKey = Vec<ScalarValue>` with `Hash + Eq` via `HashableF32`/`HashableF64` newtypes (`src/processors/scalar_value.rs`)
- `GroupByAggregate` processor: drain-and-finalize pattern with `HashMap<GroupKey, Vec<Box<dyn Aggregator>>>` (`src/processors/group_by_aggregate.rs`)
- `build_plan` wired to use `GroupByAggregate` when `group_by` is non-empty and aggregates are present
- GROUP BY columns appear first in output, followed by aggregate columns
- Analyser validates GROUP BY column names against schema

## Deferred
- HAVING clause — moved to TASK-007
- Integration tests — to be added in a later test pass
