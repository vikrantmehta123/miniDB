# TASK-004 — WHERE: Predicate Evaluator

## Description
Evaluate the `Predicate` tree from TASK-003 against a scanned granule to produce a boolean bitmask. Apply the mask to all projected columns before output. No granule skipping yet — correctness first.

---

## Status: Completed

---

## Out of Scope
- Granule skipping (TASK-005)
- Aggregation interaction with WHERE (handled when aggregations are added)
