# TASK-005 — Rayon: Parallel Column Reads Within a Part

## Description
Part-level parallelism is done (committed in d943573). Within a single part, columns are still read sequentially. Since all column files are independent, reading them in parallel is a straightforward `par_iter()` over `columns` instead of the current `for col in &columns` loop.

**Note: Lower priority than TASK-001 and TASK-006. The I/O bottleneck for a single part is disk bandwidth, not thread count — measure before investing here.**

---

## Steps

- [ ] **Extract `read_column` as a standalone function**
  - `fn read_column(part_dir: &Path, col: &ColumnDef) -> io::Result<ColumnChunk>`
  - Stateless and `Send` — no borrowed state from `FullScan`

- [ ] **Replace the sequential column loop** (`src/processors/full_scan.rs`)
  - Current: `for col in &columns { ... cols.push(chunk); }`
  - Target: `columns.par_iter().map(|col| read_column(&part_dir, col)).collect::<Result<Vec<_>, _>>()?`

- [ ] **Benchmark**
  - Wide table (6+ columns), single part
  - Compare sequential vs parallel column reads
  - Only pursue if speedup is meaningful (may be I/O bound at this scale)

---

## Out of Scope
- Granule-level parallelism within one column (memory bandwidth bound)
- Async I/O
