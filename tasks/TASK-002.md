# TASK-002 — Aggregations: AST, Parser, Pipeline Node

## Description
Make `SELECT sum(ts), count(*), max(uid) FROM events WHERE ts > 100` work end-to-end. This requires extending the AST, lowering aggregate function calls, and adding an `Aggregate` processor node to the pipeline. Wire the existing `Sum` and `Max` accumulators — the remaining ones (Min, Count, Avg) come in TASK-003.

**This is the OLAP use case. Without it the project doesn't justify the name.**

---

## Steps

### AST (`src/parser/ast.rs`)

- [X] Add `AggFunc` enum: `Sum, Max, Min, Count, Avg`
- [X] Add `SelectExpr` enum: `Col(String)` | `Agg { func: AggFunc, col: String }`
- [X] Replace `Projection::Columns(Vec<String>)` with `Projection::Exprs(Vec<SelectExpr>)`

### Parser lowering (`src/parser/lower.rs`)

- [X] Detect `Function` nodes in the SELECT list, lower to `SelectExpr::Agg`
- [X] `count(*)` → `SelectExpr::Agg { func: AggFunc::Count, col: "*".into() }`
- [X] Reject mixing `Col` and `Agg` without GROUP BY

### Analyser (`src/analyser.rs`)

- [X] Update `analyse_select` to handle `Projection::Exprs`
- [X] Validate agg column names; skip validation for `"*"`
- [X] Expand `Projection::All` into `Exprs(vec![SelectExpr::Col(...)])` for each schema column

### Aggregate processor (`src/processors/aggregate.rs`)

- [X] `Aggregate::new(input, aggs, input_idx, output_schema)`
- [X] Drain all input batches, accumulate into per-agg state, return a single 1-row `Batch`
- [X] `done` flag so second call returns `None`

### Pipeline wiring (`src/processors/mod.rs`)

- [X] `build_plan`: if projection contains any `Agg` exprs, append `Aggregate` node
- [X] `aggregator::factory::build(func, DataType)` dispatches to correct accumulator
- [ ] `Count` and `Avg` return `InvalidData` stub — complete in TASK-003

### Test

- [X] Parser test: `SELECT sum(ts), count(*), max(uid)` — assert lowered AST
- [ ] Integration test: `SELECT sum(ts) FROM events` on known rows
- [ ] Integration test: `SELECT max(uid) FROM events WHERE ok = true`

---

## Out of Scope
- Min, Count, Avg accumulators (TASK-003)
- GROUP BY (TASK-006)
