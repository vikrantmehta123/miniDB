# TASK-001 — Wire Encoders into the Write Path

## Description
Fix `codec_for()` in `table_writer.rs` so it returns the right codec per type instead of always returning `Codec::Plain`. Then add a round-trip test to lock in correctness. After this, the project can legitimately claim "Delta-encoded integer columns."

**Sprint 1 — estimated 1 session.**

---

## Steps

- [ ] **Fix `codec_for()`** (`src/storage/table_writer.rs`)
  - `i8/u8/bool` → `Codec::Plain`
  - `i16/i32/i64/u16/u32/u64` → `Codec::Delta`
  - `f32/f64` → `Codec::Plain`
  - `String` → `Codec::Plain`

---

## Out of Scope
- User-configurable codecs via `schema.json` (Phase 2)
- RLE as an option (Phase 2)
- String dictionary encoding (Phase 2)
