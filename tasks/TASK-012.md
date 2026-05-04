# TASK-012 — Background Compaction: Design Review

## Description
Before writing any compaction code, spend one session sketching the merge algorithm and concurrency model. Compaction is architecturally the riskiest task — the design must be agreed on before implementation begins. The output of this session is a decision recorded in this file and a go/no-go for Phase 1.

**Sprint 6 — estimated 1 session (design only, no code).**

---

## Steps

- [ ] **Phase decision**
  - Re-evaluate whether compaction belongs in Phase 1 or Phase 2
  - Criteria: does the project tell a better story with it? Is the complexity worth it given available time?
  - Record decision here: **In Phase 1** / **Deferred to Phase 2**

- [ ] **Sketch the merge algorithm**
  - k-way merge of N sorted parts on the primary key
  - Memory model: read one granule at a time from each input part (bounded memory)
  - Output: a new part written via `TableWriter` with default codecs

- [ ] **Sketch the concurrency model**
  - How does a reader know which parts are "stable" vs "being merged"?
  - Options: `RwLock<Vec<PartHandle>>`, generation counter, part tombstoning
  - How does a concurrent insert avoid interfering with an in-progress merge?

- [ ] **Sketch the scheduler**
  - Trigger condition: part count > threshold (e.g. 10) OR total small-part size > threshold
  - Selection strategy: smallest-N-parts-first (ClickHouse style)
  - Threading model: dedicated background thread vs `tokio` task

- [ ] **Record the design decisions here** before starting TASK-013

---

## Design Decisions (fill in during the session)

**Phase decision:**

**Merge algorithm:**

**Concurrency model:**

**Scheduler trigger and selection:**

---

## References
- ClickHouse `MergeTreeData`: `/Personal/open-source/ClickHouse/src/Storages/MergeTree/MergeTreeData.h`
- ClickHouse merger: `/Personal/open-source/ClickHouse/src/Storages/MergeTree/MergeTreeDataMergerMutator.h`
