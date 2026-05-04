# TASK-008 — Wire Aggregators into the Executor

## Description
Connect the existing aggregators (`sum`, `max`, `top_k`) to the executor so `SELECT sum(ts) FROM events` actually works end-to-end. This requires parser lowering for aggregate functions and an executor dispatch layer.

**Sprint 4 — estimated 1 session.**

---

## Steps

- [ ] **Extend the AST** (`src/parser/ast.rs`)
  - Add `SelectExpr` enum: `Column(String)` or `Agg { func: AggFunc, col: String }`
  - `AggFunc` enum: `Sum, Max, Min, Count, Avg, TopK(usize)` — start with Sum and Max
  - Replace `Projection::Columns(Vec<String>)` with `Projection::Exprs(Vec<SelectExpr>)`

- [ ] **Lower aggregate calls** (`src/parser/lower.rs`)
  - Detect `sqlparser::ast::Function` nodes in the SELECT list
  - Map `sum(col)` → `SelectExpr::Agg { func: AggFunc::Sum, col }`
  - Reject mixing bare columns and aggregates without GROUP BY (return a clear error)

- [ ] **Executor dispatch** (`src/executor.rs`)
  - If all `SelectExpr`s are `Agg` variants: route to an aggregation path
  - Aggregation path: scan the required column(s), pass each granule's values to the accumulator, call `finalize()`, print the result
  - Mix of bare columns + aggs without GROUP BY → error (GROUP BY is TASK-010)

- [ ] **Integration test**
  - Insert known rows, `SELECT sum(ts) FROM events` — verify the sum
  - `SELECT max(uid) FROM events` — verify the max

---

## Out of Scope
- MIN, COUNT, AVG (TASK-009)
- GROUP BY (TASK-010)
- Interaction with WHERE (should work for free once TASK-004 is done)
