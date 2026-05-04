# TASK-004 — WHERE: Predicate Evaluator

## Description
Evaluate the `Predicate` tree from TASK-003 against a scanned granule to produce a boolean bitmask. Apply the mask to all projected columns before output. No granule skipping yet — correctness first.

**Sprint 2 — estimated 1–2 sessions.**

---

## Steps

- [ ] **Bitmask type**
  - Use `Vec<bool>` (or `bitvec` crate) as the row-level filter mask
  - Length = number of rows in the current granule

- [ ] **Leaf evaluation**
  - For `Predicate::Cmp { col, op, value }`: scan the column's decoded values for the granule, compare each against `value`, produce a `Vec<bool>`
  - Support all `CmpOp` variants for i64, u64, f64, bool, String
  - Type mismatch (e.g. comparing a string column to an integer literal) → `Err`

- [ ] **Compound evaluation**
  - `And`: element-wise `&` of two masks
  - `Or`: element-wise `|`
  - `Not`: element-wise `!`

- [ ] **Apply mask in executor**
  - After scanning all columns in a granule, apply the mask: keep only rows where mask is `true`
  - Wire into the existing scan loop in the executor

- [ ] **Integration test**
  - Insert 3 parts of known rows
  - `SELECT * FROM t WHERE ts > X` — verify only matching rows returned
  - `SELECT * FROM t WHERE uid = 1 AND ok = true` — compound predicate test

---

## Out of Scope
- Granule skipping (TASK-005)
- Aggregation interaction with WHERE (handled when aggregations are added)
