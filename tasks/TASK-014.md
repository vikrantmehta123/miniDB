# TASK-014 — Background Compaction: Scheduler and Reader Integration

## Description
Wire the merge algorithm from TASK-013 into a background thread that monitors part count and triggers merges automatically. Update `TableReader` to use a shared, lock-guarded part list so readers never see a mid-merge part.

**Sprint 6 — estimated 2 sessions.**

---

## Steps

- [ ] **Shared part list**
  - Wrap the part directory listing in `Arc<RwLock<Vec<PartHandle>>>`
  - `TableReader` acquires a read lock at the start of each query, clones the list, releases immediately — reads are never blocked by the lock itself
  - `TableWriter` acquires a write lock only to append a new part handle after INSERT

- [ ] **Part tombstoning**
  - When the merger selects a merge set, mark those parts as `being_merged` in the shared list
  - Readers that cloned the list before the mark see old parts — fine, they finish normally
  - Readers that clone after the mark skip tombstoned parts

- [ ] **Merge scheduler** (`src/compaction/scheduler.rs`)
  - Spawn a `std::thread` (or `tokio` task) that polls the part list on a timer
  - Trigger condition: part count exceeds `MERGE_THRESHOLD` (e.g. 10); make it a constant in `config.rs`
  - Selection: pick the N smallest parts (by total byte size across all columns)
  - Send a `MergeJob { parts: Vec<PartHandle> }` over a `std::sync::mpsc` channel to a worker thread

- [ ] **Worker thread**
  - Receives `MergeJob`, calls `merger::merge(job.parts, output_dir)`
  - On success: updates the shared part list (remove tombstoned, add merged)
  - On failure: clears the tombstones, logs the error

- [ ] **Graceful shutdown**
  - On process exit (Ctrl-D in REPL): send a shutdown signal to the scheduler, wait for any in-flight merge to finish before exiting

- [ ] **End-to-end test**
  - Insert 12 parts (12 separate INSERT calls)
  - Wait for the scheduler to trigger; verify part count drops
  - Run a full scan and verify all rows are still present and in sorted order

---

## Out of Scope
- Multiple concurrent merge workers
- Merge throttling (rate limiting to avoid starving queries)
- External merge sort (Phase 2)
