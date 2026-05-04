# TASK-009 — Aggregators: MIN, COUNT, AVG

## Description
Add the three remaining essential aggregators. Each is a standalone accumulator in `src/aggregator/`. After this, the five SQL standard aggregates (SUM, MAX, MIN, COUNT, AVG) all work.

**Sprint 4 — estimated 1 session.**

---

## Steps

- [ ] **`min`** (`src/aggregator/min.rs`)
  - Mirror of the existing `max` accumulator
  - Tracks the running minimum; `finalize()` returns it
  - Wire `AggFunc::Min` in the executor dispatch (TASK-008)

- [ ] **`count`** (`src/aggregator/count.rs`)
  - Increments a `u64` counter for each non-null row
  - `COUNT(*)` and `COUNT(col)` are equivalent for now (no nulls in Phase 1)
  - Wire `AggFunc::Count` in the executor

- [ ] **`avg`** (`src/aggregator/avg.rs`)
  - Maintains a running `sum: f64` and `count: u64`
  - `finalize()` returns `sum / count as f64`; return `None` on empty input
  - Wire `AggFunc::Avg` in the executor

- [ ] **Tests**
  - One test per aggregator: known input, assert correct output
  - Edge case: empty table → `count` returns 0, `min/max/avg` return `None` or a sentinel

---

## Out of Scope
- Approximate aggregations (HyperLogLog, quantiles — deferred)
- `first` / `last` accumulators (deferred)
- GROUP BY variants (TASK-010)
