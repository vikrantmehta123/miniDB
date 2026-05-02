# tinyOLAP — Columnar Database in Rust

## Project Goals
1. **Learn Rust by doing** — concurrency, ownership, safety, traits, async
2. **Build a columnar database from scratch** — understand how real column stores work. We more or less try to look at ClickHouse for inspiration. The source code for ClickHouse can be found at /Personal/open-source/ClickHouse.

## Core Design Decisions
- **Columnar storage**: data stored column-by-column, not row-by-row
- **Granule size**: `GRANULE_SIZE = 512` values (the atomic addressable unit — one mark per granule). Configured in `src/config.rs`.
- **Block buffer**: `BLOCK_BUFFER_SIZE = 8 KiB` of uncompressed bytes per compressed block (multiple granules may share a block; one block = one lz4 compress call).
- **Supported types**: all numeric types (`i8`, `i16`, `i32`, `i64`, `u8`, `u16`, `u32`, `u64`, `f32`, `f64`), `bool`, variable-length strings
- **Insert API is column-oriented**: `TableWriter::insert(Vec<ColumnChunk>)` — the caller transposes rows into columns. `ColumnChunk` is an enum with one variant per supported type.
- **One INSERT = one part**: parts are immutable directories `part_NNNNN/` containing per-column `<col>.bin` (compressed data) and `<col>.mrk` (granule index). Writes go to `tmp_part_NNNNN/` and are atomically renamed on success.
- **Encoding library** (`src/encoding/`): standalone codecs (`Plain`, `Delta`, `RLE`) over a sealed `Primitive` trait, dispatched by the `Codec` enum. Not yet wired into column writers.
- **Disk-persistent**: data lives on disk, not in memory — no in-memory-only database
- **No custom parser**: use an off-the-shelf SQL parser crate
- **Features evolve as we write code** — don't over-plan

## What to Emphasize as We Build
- Use Rust idioms: `Result`/`Option` over panics, iterators over loops where natural
- Prefer safe code; use `unsafe` only when necessary and document why
- Leverage Rust's type system to model the column type system (e.g., enums + generics)
- Introduce concurrency (e.g., `rayon`, channels, `Arc<Mutex<>>`) when it fits naturally
- Keep it simple — no premature abstractions

## Current Task
- Active work is tracked in `TASK.md` at the repo root. Always read it at the start of a session to know what we're building next and which step we're on.
- `TASK.md` contains only what is actively being worked on — completed steps are removed. It is not a history log.
- Feature scope and phasing is in `SPEC.md` at the repo root. Read it for context on what Phase 1 covers and what is deferred to Phase 2.

## Build System
- Standard `cargo` — `cargo build`, `cargo run`, `cargo test`
- No special flags needed beyond what Cargo.toml specifies

## Style
- Move fast, keep it simple — this is a learning project, not production software
- Prefer clarity over cleverness unless the clever version teaches something about Rust
- Tests are good when they lock in correctness of a tricky piece; don't test everything

## Collaboration Rules
- **Claude must NOT write code to files.** The user types every line themselves.
- Claude's role: explain concepts, show code snippets in chat, guide decisions, answer questions.
- Exceptions: documentation/docstrings only (e.g., updating CLAUDE.md).
