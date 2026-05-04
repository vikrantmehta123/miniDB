# TASK-005 — Rayon: Parallel Column Reads Within a Part

## Description
Within a single part, all columns are independent files. Read them in parallel using `rayon`. Combines with TASK-004 to give two levels of parallelism: across parts and across columns.

**Prerequisite: TASK-004.**

---

## Steps

- [ ] **Identify the parallelism boundary** (`src/processors/full_scan.rs`)
  - Currently: for each part, columns are read in a sequential `for col in &self.columns` loop
  - Target: replace with `self.columns.par_iter().map(|col| read_column(...)).collect::<Result<Vec<_>, _>>()`

- [ ] **Extract `read_column` as a standalone function**
  - `fn read_column(part_dir: &Path, col: &ColumnDef) -> io::Result<ColumnChunk>`
  - Must be stateless and `Send` — no borrowed state from `FullScan`

- [ ] **Combine with part-level parallelism**
  - Outer: `parts.par_iter()` (TASK-004)
  - Inner: `columns.par_iter()` per part
  - Both levels active simultaneously

- [ ] **Benchmark**
  - Wide table (6+ columns), many parts
  - Compare: sequential vs part-parallel vs part+column parallel
  - Record numbers here

---

## Out of Scope
- Granule-level parallelism within one column (memory bandwidth bound — not worth it)
- Async I/O
