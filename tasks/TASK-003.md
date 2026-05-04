# TASK-003 — Aggregators: MIN, COUNT, AVG

## Description
Add the three remaining standard SQL aggregators as standalone accumulators in `src/aggregator/`, then wire them into the `Aggregate` processor built in TASK-002. After this, all five SQL standard aggregates work.

**Prerequisite: TASK-002 must be done.**

---

## Steps

### `min` (`src/aggregator/min.rs`)

- [ ] Mirror of the existing `Max<T>` — `T: Copy + Ord`, state is `Option<T>`
- [ ] `init() -> Option<T>`, `update`, `merge`, `finalize` — same structure as `Max`
- [ ] Separate `MinFloat<T>` for `f32`/`f64` (uses `PartialOrd`, skips NaN) — mirror of `MaxFloat`
- [ ] Tests: basic, empty, negatives, merge

### `count` (`src/aggregator/count.rs`)

- [ ] State is `u64`; increments by `input.len()` on each `update`
- [ ] `COUNT(*)` and `COUNT(col)` are equivalent for now (no nulls in Phase 1)
- [ ] `finalize` returns the `u64` count
- [ ] Tests: empty → 0, multi-chunk

### `avg` (`src/aggregator/avg.rs`)

- [ ] State is `(sum: f64, count: u64)`
- [ ] `update`: accumulate sum as `f64`, increment count
- [ ] `finalize`: return `sum / count as f64`; return `None` on empty input
- [ ] Tests: known values, empty, merge across chunks

### Wire into `Aggregate` processor (`src/processors/aggregate.rs`)

- [ ] Replace the `not yet implemented` stubs from TASK-002 with real dispatch to the new accumulators
- [ ] `Min` dispatches to `min::Min<T>` or `min::MinFloat<T>` based on column type
- [ ] `Count` dispatches to `count::Count`
- [ ] `Avg` dispatches to `avg::Avg`

### Tests

- [ ] `SELECT min(ts), count(*), avg(uid) FROM events` on known rows — verify all three values
- [ ] `SELECT count(*) FROM events WHERE ok = false` — filter + count

---

## Out of Scope
- `first` / `last` accumulators (deferred)
- Approximate aggregations: HyperLogLog, quantiles (deferred to Deferred.md)
- GROUP BY variants (TASK-006)
