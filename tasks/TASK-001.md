# TASK-001 — Zone Maps: Granule Skipping

## Description
During INSERT, compute `(min, max)` per granule per column and write a `.zonemap` file alongside `.bin` and `.mrk`. During scan, load zone maps and skip granules where the predicate provably cannot match. This turns the current "scan everything" `FullScan` into a real columnar optimizer.

**This is the highest-priority task.** It's the difference between "has a WHERE clause" and "has a scan optimizer" — the latter is the interview talking point.

---

## Steps

### Write path (`src/storage/column_writer.rs`)

- [ ] Track `min: T` and `max: T` within each granule as values are appended to `block_values`
- [ ] On each granule boundary (where a mark is pushed), record `(min, max)` into a `Vec<(T, T)>`
- [ ] At the end of `write_column`, serialize each `(min, max)` pair as two `T::WIDTH`-byte LE values and write to `<col>.zonemap`
- [ ] `flush_block` doesn't need to change — zone map entries are per granule, not per block

### Read path (`src/storage/column_reader.rs`)

- [ ] On `ColumnReader::open`, load `<col>.zonemap` into `Vec<(i64, i64)>` — cast all types to i64 for uniform storage; floats use bit-cast or are left as plain f64 stored as u64 bits
- [ ] Implement `can_skip(granule_idx: usize, op: &CmpOp, value: i64) -> bool`
  - `Eq`: skip if `value < min || value > max`
  - `Ne`: never skip (could match anything)
  - `Lt`: skip if `min >= value`
  - `Le`: skip if `min > value`
  - `Gt`: skip if `max <= value`
  - `Ge`: skip if `max < value`

### FullScan integration (`src/processors/full_scan.rs`)

- [ ] Change granularity: instead of reading a whole part per `next_batch()`, iterate granule by granule
- [ ] Before reading each granule: for each predicate leaf that touches a column in this part, call `can_skip`; if all agree → skip
- [ ] `Batch` emitted per surviving granule (or group them — one part is also fine for now)
- [ ] Expose a `granules_skipped` counter (can just `eprintln!` it for now; useful for debugging)

### Test

- [ ] Insert rows such that some granules are entirely above / below a threshold
- [ ] Run a selective WHERE and assert the correct rows come back
- [ ] Add a counter check: verify at least one granule was skipped

---

## Out of Scope
- Zone maps for string columns (length-based min/max is possible but deferred)
- Bloom filters (deferred)
- Multi-column predicate pruning via AND/OR across zone maps
