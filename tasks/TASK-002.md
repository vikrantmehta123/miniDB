# TASK-002 — Aggregations: AST, Parser, Pipeline Node

## Description
Make `SELECT sum(ts), count(*), max(uid) FROM events WHERE ts > 100` work end-to-end. This requires extending the AST, lowering aggregate function calls, and adding an `Aggregate` processor node to the pipeline. Wire the existing `Sum` and `Max` accumulators — the remaining ones (Min, Count, Avg) come in TASK-003.

**This is the OLAP use case. Without it the project doesn't justify the name.**

---

## Steps

### AST (`src/parser/ast.rs`)

- [ ] Add `AggFunc` enum: `Sum, Max, Min, Count, Avg`
- [ ] Add `SelectExpr` enum:
  - `SelectExpr::Col(String)` — a plain column reference
  - `SelectExpr::Agg { func: AggFunc, col: String }` — an aggregate call; `col` is `"*"` for `COUNT(*)`
- [ ] Replace `Projection::Columns(Vec<String>)` with `Projection::Exprs(Vec<SelectExpr>)`
  - `Projection::All` stays as-is; it expands to plain `Col` expressions in the analyser

### Parser lowering (`src/parser/lower.rs`)

- [ ] In `lower_projection`, detect `sqlparser::ast::Function` nodes and lower them to `SelectExpr::Agg`
  - Map `sum(col)` → `AggFunc::Sum`, `max(col)` → `AggFunc::Max`, etc.
  - `count(*)` → `SelectExpr::Agg { func: AggFunc::Count, col: "*".into() }`
- [ ] Plain `Identifier` items still lower to `SelectExpr::Col`
- [ ] Reject mixing `Col` and `Agg` without `GROUP BY` — return a clear `ParseError`

### Analyser (`src/analyser.rs`)

- [ ] Update `analyse_select` to handle the new `Projection::Exprs` variant
- [ ] For each `SelectExpr::Agg`, validate the column exists in the schema (skip for `"*"`)
- [ ] Expand `Projection::All` into `Exprs(vec![SelectExpr::Col(name) for each schema column])`

### Aggregate processor (`src/processors/aggregate.rs`)

- [ ] `Aggregate::new(input: Box<dyn Processor>, exprs: Vec<SelectExpr>) -> Self`
- [ ] `next_batch()`: drain all batches from `input`, accumulate into per-expr state, return a single `Batch` with one row on the first call, `None` on the second
- [ ] Dispatch per `AggFunc`:
  - `Sum` → use `aggregator::sum::Sum<T>` for the column's concrete type
  - `Max` → use `aggregator::max::Max<T>` (or `MaxFloat` for f32/f64)
  - `Min`, `Count`, `Avg` → stub returning `Err(ExecutionError::InvalidData("not yet implemented"))` until TASK-003

### Pipeline wiring (`src/processors/mod.rs`)

- [ ] In `build_plan`: if the projection contains any `Agg` exprs, append an `Aggregate` node as the final stage (after `Filter`, before returning)
- [ ] Pass `scan_cols` correctly: agg input columns must be in the scan set even if not in the output projection

### Executor (`src/executor.rs`)

- [ ] No structural change needed — the pipeline change is transparent. Update the result printing in `main.rs` if needed.

### Test

- [ ] `SELECT sum(ts) FROM events` on known rows — verify the sum
- [ ] `SELECT max(uid) FROM events WHERE ok = true` — verify filter + agg compose correctly

---

## Out of Scope
- Min, Count, Avg accumulators (TASK-003)
- GROUP BY (TASK-006)
- Multiple aggregates in one query (can stub or implement — your call)
