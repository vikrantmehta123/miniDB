# TASK-007 — Rayon: Granule-Level Parallelism

## Description
Within a single part, parallelize column reads across columns (not across granules — granules within one column are sequential for cache locality). Multiple columns can be decompressed and decoded independently in parallel.

**Sprint 3 — estimated 1 session.**

---

## Steps

- [ ] **Identify the parallelism boundary**
  - The per-part scan reads N columns; each column's granules are read sequentially
  - Columns are independent: `ts.bin` and `uid.bin` share no state
  - Parallelize across `projection.columns` using `par_iter()`

- [ ] **Refactor per-column read into a standalone function**
  - Extract `read_column(part_dir, col_def) -> io::Result<ColumnChunk>` if not already isolated
  - Must be `Send + 'static` or use a scoped rayon thread pool

- [ ] **Combine with part-level parallelism (TASK-006)**
  - Outer: `parts.par_iter()` (from TASK-006)
  - Inner: `columns.par_iter()` per part
  - Result: a `Vec<Vec<ColumnChunk>>` — indexed by `[part][column]`, merged in order

- [ ] **Benchmark**: measure speedup on a wide table (4+ columns) vs single-threaded

---

## Out of Scope
- Async I/O (not in Phase 1)
- Granule-level parallelism within one column (memory bandwidth bound, likely not worth it)
