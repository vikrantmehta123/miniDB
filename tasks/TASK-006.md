# TASK-006 — GROUP BY with Hash Aggregation

## Description
Implement `GROUP BY` using a `HashMap` (via the existing `ahash` dependency) to partition rows into per-key accumulator buckets. This is the most algorithmically complex task in Phase 1 and the one interviewers at DB companies ask about most.

**Prerequisite: TASK-002 and TASK-003 (all five aggregators must work).**

---

## Steps

### AST + Parser (`src/parser/ast.rs`, `lower.rs`)

- [ ] Add `group_by: Vec<String>` to `SelectStmt`
- [ ] In `lower_select`: detect the GROUP BY clause, extract column names
- [ ] Validation: if GROUP BY is present, every `SelectExpr::Col` in the projection must appear in the GROUP BY list — error otherwise

### Key type

- [ ] Define `GroupKey(Vec<KeyValue>)` where `KeyValue` is a stripped-down `Literal` without `Null`
- [ ] Implement `Hash` and `Eq` for `GroupKey` (derive or manual)
- [ ] Extract a `GroupKey` from a batch row index given the GROUP BY column chunks

### `GroupByAggregate` processor (`src/processors/group_by.rs`)

- [ ] `GroupByAggregate::new(input: Box<dyn Processor>, group_cols: Vec<String>, agg_exprs: Vec<SelectExpr>) -> Self`
- [ ] State: `AHashMap<GroupKey, Vec<AccumulatorState>>`
  - One `AccumulatorState` enum variant per `AggFunc` (holds the typed state)
- [ ] `next_batch()`:
  - First call: drain all batches from `input`; for each row, extract `GroupKey`, look up or insert accumulator slot, feed the value; then finalize all groups and return a `Batch` with one row per group
  - Second call: return `None`
- [ ] Column order in output: GROUP BY columns first, then aggregate results

### HAVING clause

- [ ] Parse `HAVING agg_func(col) op literal` — same grammar as WHERE, post-aggregation
- [ ] Apply as a filter on the finalized `Batch` before returning it

### Pipeline wiring (`src/processors/mod.rs`)

- [ ] In `build_plan`: if `group_by` is non-empty, use `GroupByAggregate` instead of `Aggregate`

### Tests

- [ ] `SELECT uid, sum(ts) FROM events GROUP BY uid` — known rows, verify per-uid sums
- [ ] `SELECT ok, count(*) FROM events GROUP BY ok HAVING count(*) > 1`
- [ ] Single-group edge case: `GROUP BY` on a column with one distinct value

---

## Out of Scope
- ORDER BY on grouped results
- Parallel hash aggregation
- Multi-level GROUP BY
