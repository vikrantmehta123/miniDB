# TASK-013 — Background Compaction: Merge Algorithm

## Description
Implement the k-way merge that combines N sorted parts into one larger sorted part. This task covers only the merge logic — the scheduler and reader integration are TASK-014. Prerequisite: TASK-012 design decisions must be recorded.

**Sprint 6 — estimated 2 sessions.**

---

## Steps

- [ ] **Create `src/compaction/` module**
  - `mod.rs` re-exports public types
  - `merger.rs` contains the merge algorithm
  - `scheduler.rs` is TASK-014

- [ ] **k-way merge** (`src/compaction/merger.rs`)
  - Accept a `Vec<PartHandle>` (the parts to merge) and an output part directory
  - Open a `ColumnReader` per column per input part
  - Use a min-heap keyed on the primary key column value to drive the merge
  - Drain one row at a time from the heap into output buffers
  - When output buffers reach `GRANULE_SIZE`, flush via `TableWriter`
  - On completion: atomically rename the output `tmp_part_NNNNN/` → `part_NNNNN/`

- [ ] **Atomicity on failure**
  - If the merge fails mid-way: delete `tmp_part_NNNNN/`, leave source parts untouched
  - Source parts are deleted only after the rename succeeds

- [ ] **Test**
  - Write 3 parts of interleaved but globally sorted rows (sorted by primary key within each part)
  - Run the merge algorithm directly (no scheduler)
  - Verify the output part contains all rows in globally sorted order
  - Verify source parts are deleted and the merged part is readable

---

## Out of Scope
- Scheduler (TASK-014)
- Reader integration (TASK-014)
- External merge sort for parts exceeding memory (Phase 2)
