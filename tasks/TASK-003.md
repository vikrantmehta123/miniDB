# TASK-003 — Aggregators: COUNT and AVG

## Description
Complete the five standard SQL aggregates. MIN is already done. Count and Avg are stubbed in `factory.rs` returning `InvalidData`. This task wires them in.

**Prerequisite: TASK-002 is done.**

---

## Steps

### `count` (`src/aggregator/count.rs`)

- [ ] State is `u64`; `update` increments by `chunk.len()`
- [ ] `COUNT(*)` and `COUNT(col)` equivalent for now (no nulls in Phase 1)
- [ ] `finalize` returns the `u64` as a `ColumnChunk::U64`
- [ ] `output_type()` returns `DataType::U64`
- [ ] Tests: empty → 0, multi-chunk, merge

### `avg` (`src/aggregator/avg.rs`)

- [ ] State is `(sum: f64, count: u64)`
- [ ] `update`: add each value cast to f64, increment count
- [ ] `finalize`: return `sum / count` as `ColumnChunk::F64`; return `0.0` on empty (or `None` if you add a sentinel — your call)
- [ ] `output_type()` returns `DataType::F64`
- [ ] Tests: known values, empty, merge across chunks

### Wire into factory (`src/aggregator/factory.rs`)

- [ ] Replace the `Count | Avg => Err(...)` stub with real dispatch
- [ ] `Count` → `Box::new(CountAgg::new())` (no DataType needed — type-blind)
- [ ] `Avg` → `Box::new(AvgAgg::new())` (accumulates as f64 regardless of input type)

### Integration tests

- [ ] `SELECT count(*) FROM events` — verify row count
- [ ] `SELECT avg(uid) FROM events WHERE ok = true` — filter + avg
- [ ] `SELECT sum(ts), count(*), min(ts), max(ts), avg(ts) FROM events` — all five in one query

---

## Out of Scope
- `first` / `last` accumulators (deferred)
- Approximate aggregations: HyperLogLog, quantiles (Deferred.md)
- GROUP BY variants (TASK-006)
