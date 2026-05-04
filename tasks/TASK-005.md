# TASK-005 — WHERE: Granule Skipping via Zone Maps

## Description
During INSERT, compute `(min, max)` per granule per column and persist as a zone map file. At scan time, load zone maps and skip granules where the predicate cannot match. This is the core columnar optimization — the difference between "scans everything" and "a real column store."

**Sprint 2 — estimated 1–2 sessions.**

---

## Steps

- [ ] **Zone map format**
  - One `<col>.zonemap` file per column per part
  - Fixed-width records: `[min: T as 8 bytes][max: T as 8 bytes]` per granule
  - For strings: store min/max byte length (skip value-level pruning for now)
  - For bool: store `any_true` and `any_false` flags

- [ ] **Compute and write zone maps during INSERT** (`src/storage/column_writer.rs`)
  - Track `min` and `max` within each granule as values are appended
  - On granule boundary: write the `(min, max)` record
  - Flush at the end of the column write

- [ ] **Load zone maps during scan** (`src/storage/column_reader.rs`)
  - On `ColumnReader::open`, read the `.zonemap` file into a `Vec<(min, max)>`
  - Expose `can_skip_granule(granule_idx: usize, predicate: &Predicate) -> bool`

- [ ] **Skip in the executor scan loop**
  - Before reading a granule, call `can_skip_granule` for each predicate leaf touching that column
  - If all predicate columns say skip → advance past the granule without decompressing
  - Count skipped granules (log or expose as a stat)

- [ ] **Benchmark**: re-run the TASK-002 scan benchmark with a selective WHERE and measure granules read vs skipped

---

## Out of Scope
- Bloom filters (Phase 2)
- Primary index (`primary.idx`) — separate task if/when added
- Zone maps for compound predicates (AND/OR pruning across multiple columns)
