# TASK-002 — Aggregations: AST, Parser, Pipeline Node

## Status: Completed

## What was built
- `AggFunc`, `SelectExpr`, `Projection::Exprs` in `ast.rs`
- Full parser lowering for `sum(col)`, `min(col)`, `max(col)`, `avg(col)`, `count(col)`
- `Aggregate` processor node (drain-and-finalize pattern)
- `aggregator::factory::build(func, DataType)` factory
- `build_plan` wires `Aggregate` when agg exprs are present
- All five standard SQL aggregates fully working end-to-end
- Parser and integration tests passing

## Notes
- `count(*)` / `count()` are intentionally not supported — only `count(col)`
- Mixing `Col` and `Agg` without GROUP BY is rejected at the parser layer
