# TASK-010 — GROUP BY with Hash Aggregation

## Description
Implement `GROUP BY` using a hash map to partition rows into per-key accumulator states. This is the most algorithmically complex task in Phase 1 and the one interviewers will ask about.

**Sprint 4 — estimated 2–3 sessions.**

---

## Steps

- [ ] **Parser lowering** (`src/parser/lower.rs`)
  - Detect `GROUP BY col1, col2` in the sqlparser-rs AST
  - Carry `group_by: Vec<String>` in `Statement::Select`
  - Validate: if GROUP BY is present, every non-aggregate SELECT expression must appear in the GROUP BY list — error otherwise

- [ ] **Key type**
  - Define `GroupKey` as a `Vec<Literal>` (one entry per GROUP BY column)
  - Implement `Hash` and `Eq` for `GroupKey` so it can be used as a `HashMap` key

- [ ] **Keyed accumulator map**
  - `HashMap<GroupKey, Vec<Box<dyn Accumulator>>>` — one accumulator per aggregate function per group
  - On each row: extract the key from the GROUP BY columns, look up (or insert) the accumulator slot, feed the row's aggregate-column value

- [ ] **Executor: GROUP BY path**
  - Scan all required columns (GROUP BY columns + aggregate-input columns) in lock-step
  - For each row: compute `GroupKey`, dispatch to accumulators
  - After all parts: drain the map, call `finalize()` per group, emit one result row per group

- [ ] **HAVING clause**
  - Parse `HAVING agg_func(col) op literal` (same grammar as WHERE but post-aggregation)
  - Apply as a filter on the finalized rows before output

- [ ] **Integration tests**
  - `SELECT uid, sum(ts) FROM events GROUP BY uid` — known rows, verify per-uid sums
  - `SELECT ok, count(*) FROM events GROUP BY ok HAVING count(*) > 1`

---

## Out of Scope
- Multi-level GROUP BY optimization (streaming vs hash)
- ORDER BY on aggregated results
- Parallel hash aggregation (Phase 2)
