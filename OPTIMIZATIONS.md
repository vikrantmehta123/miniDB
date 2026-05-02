# tinyOLAP — Storage Optimizations & Hardening Notes

This is a long-form review of the storage layer (writer + reader) with worked
examples. The goal is intuition: *why* a change matters, not just *what* to
change. Encoding optimizations (FOR / delta / RLE / dictionary) are tracked
separately and are out of scope here.

Workload assumptions baked in:
- Inserts are bounded — at most a few thousand rows, worst case ~100k, ~1 MB
  of data per `insert()` call.
- A part is atomic per insert (one `insert()` produces one finalized part).
- Reads are batch scans / aggregations, not point lookups.

---

## 1. Correctness & Durability

### 1.1 fsync — currently missing, must add

The writer today does `BufWriter::flush()` and then returns `Ok`. That only
pushes bytes from the user-space buffer into the **OS page cache**. If the
machine loses power one second later, the OS hasn't written anything to the
SSD yet — your "successful" insert is gone, and you may end up with a
half-written part directory that the reader will choke on.

`fsync` (via `File::sync_all`) is the syscall that says: "flush this file's
dirty pages and metadata down to stable storage and don't return until the
device confirms." Until you call it, you have no durability guarantee.

For a part-atomic format you need three fsyncs per insert:

1. `bin.sync_all()` — every column's `.bin` file
2. `mrk.sync_all()` — every column's `.mrk` file
3. After `fs::rename(tmp_dir → part_dir)`, open the **table directory** and
   `sync_all()` it. This is the part people forget. The rename is recorded
   as a directory entry change; on ext4/xfs that change can sit in cache
   too. Without fsyncing the directory, a crash can leave the part files on
   disk but no directory entry pointing at them.

The order matters: fsync the *contents* before the rename, then fsync the
*directory* after. That gives you "either the part is fully there, or it
isn't" — which is the atomicity guarantee you said you want.

Cost: a few ms per part on SSD. For ~1 MB inserts this is the dominant cost,
but it's the price of durability. A future optimization is a group-commit
WAL where many inserts share one fsync.

### 1.2 `stream_position()` on `BufWriter` — minor, can keep as-is

Original concern: in [column_writer.rs:75](src/column_writer.rs#L75) you
call `self.bin.stream_position()`. `BufWriter` *does* track buffered bytes
correctly (it adds the buffered byte count to the underlying file's
position), so this returns the right value. There's no bug.

**Why I flagged it anyway:** it's a small consistency wart. The string
writer tracks `block_offset` manually as a `u64` field. The numeric writer
asks the `BufWriter` for it. Both produce the same answer, but two different
mechanisms means two different things to test and two different ways to
introduce a future bug.

Worked example of where it could bite — *if* the writer ever changes:
```text
// Imagine someone refactors flush_block to compress in chunks:
self.bin.write_all(&header)?;        // buffered
let pos = self.bin.stream_position()?; // <-- now BufWriter says (file_pos + 4)
                                       // but is that what we want as the
                                       // "block_offset"? Header or post-header?
self.bin.write_all(&payload)?;
```
Manual tracking forces you to write `self.block_offset += header.len() +
payload.len();` after the writes — which is explicit about the contract.
`stream_position()` hides that, which is fine until it isn't.

**Verdict given your constraints:** leave it. It works. Just be aware that
the two writers have a stylistic mismatch.

### 1.3 `metadata().len()` for `bin_bytes` — concrete failure mode

In [column_writer.rs:97](src/column_writer.rs#L97):
```rust
self.bin.flush()?;
let bin_bytes = self.bin.get_ref().metadata()?.len();
```

`metadata()` calls `fstat()`. The size returned is the file's logical size
*as the kernel currently reports it*. Two issues:

1. **It's a syscall** — and one you don't need. After the final `flush()`,
   the position you've been tracking (`stream_position()` or your manual
   counter) already equals the file size. Just use that.
2. **Subtle race**: on filesystems where `mmap` and `write` interact (or on
   network filesystems like NFS) a `stat` immediately after a `write` has
   been observed to lag. Unlikely on a local ext4 SSD, but it's a
   "phantom-correct" pattern — works 99.9% of the time, fails weirdly.

Worked example of the difference:
```text
WriterA: writes 1000 bytes, flushes, calls metadata().len()  → 1000  ✓
WriterB: writes 1000 bytes, flushes, calls stream_position() → 1000  ✓ (free)
```
Same answer, but WriterB needs no extra syscall and doesn't depend on the
filesystem's stat consistency.

**Action:** replace `metadata().len()` with the position you're already
tracking. After `flush()`, `bin.stream_position()?` is correct and free.

### 1.4 `u32` length prefix — not a bug at your scale

`compress_prepend_size` writes a `u32` length prefix, and you write your own
`u32` framing on top. A single block can't exceed ~64 KB (your
`BLOCK_BUFFER_SIZE * 2` cap), so 4 bytes is fine forever. Just leave a
`debug_assert!(compressed.len() <= u32::MAX as usize)` so future-you doesn't
silently truncate after a refactor.

### 1.5 `MarkWriter` buffers everything in memory

`MarkWriter::write` extends an in-memory `Vec<u8>` and only writes on
`flush()`. At 24 bytes per mark and ~100 marks per 1 MB insert, this is 2.4
KB — totally fine for your workload. Skip.

### 1.6 Cache `TableDef` in `TableWriter`

`TableWriter::open` reads the schema once. Good. `TableWriter::insert`
doesn't re-read it (I misread earlier). No change needed.

### 1.7 Concurrent writer race on `next_part_id`

`AtomicU32::fetch_add` makes part-id allocation safe *within* a single
`TableWriter`. Two `TableWriter` instances (two processes, or two opens in
the same process) can both pick id `42` and clobber each other.

Worked example:
```text
Process A: TableWriter::open  → scans dir, finds max=41, next=42
Process B: TableWriter::open  → scans dir, finds max=41, next=42
Process A: insert → writes tmp_part_00042 → renames to part_00042
Process B: insert → writes tmp_part_00042 → renames to part_00042
                    └── overwrites A's part! Data loss.
```
Fix options, in increasing rigor:
- Document "single writer per table" and crash if a second one opens.
- Use a lockfile (`flock` on a `table.lock` file) — one syscall, blocks
  multiple writers cleanly.
- Centralize id allocation in a manifest file with atomic CAS.

For tinyOLAP, a lockfile is the right call.

---

## 2. Performance — Hot Path Allocations

This is the single biggest win in the codebase right now. Worth understanding
deeply.

### 2.1 `to_le_bytes_vec(&self) -> Vec<u8>` — per-value heap allocation

Look at the trait:
```rust
fn to_le_bytes_vec(&self) -> Vec<u8> {
    self.to_le_bytes().to_vec()
}
```

`to_le_bytes()` returns `[u8; 8]` for an `i64` — a stack array, free. Then
`.to_vec()` allocates an 8-byte heap buffer, memcpys the array into it,
returns a `Vec<u8>` that owns that heap allocation. The caller then does
`extend_from_slice(&v.to_le_bytes_vec())` which copies *again* into
`block_buf` and drops the temporary `Vec`, freeing the 8-byte allocation.

**Per `i64` value, today:**
- 1 stack-to-heap copy (8 bytes)
- 1 `malloc(8)` + 1 `free(8)`
- 1 heap-to-buffer copy (8 bytes)

**Cost at 100k rows:** 100k mallocs + 100k frees, plus two redundant 8-byte
copies. `malloc` is ~10–50 ns on a good allocator, so you're burning
1–5 ms of pure allocator overhead before any real work. On a column where
the actual work (memcpy + LZ4) takes ~200 µs, allocator noise is dominating
the column-write time.

**Fix — option A: write to a writer.** Change the trait:
```rust
fn write_le<W: Write>(&self, w: &mut W) -> io::Result<()>;
```
Each impl just calls `w.write_all(&self.to_le_bytes())`. Zero allocations.

**Fix — option B: bulk memcpy via `bytemuck`.** The real insight: for a slice
of `i64`s on a little-endian machine, `&[i64]` and `&[u8]` of length `8 *
N` are byte-identical. `bytemuck::cast_slice(values): &[u8]` reinterprets
the slice with no copy and no allocation. Then `block_buf.extend_from_slice
(bytes)` is *one* memcpy for the whole chunk.

Worked example, 1024 `i64` values (one granule):
```text
Today:                     Bulk:
  1024 mallocs               0 mallocs
  1024 frees                 0 frees
  1024 × 8B copy (to Vec)    0
  1024 × 8B copy (extend)    1 × 8192B memcpy   ← one memcpy
  ────────────────           ─────────────
  ~30 µs allocator           ~1 µs total
  + 30 µs of copies
```
That's ~50× faster for the per-granule write path. Multiply by columns and
granules and you're talking real wall-clock.

The catch: `bytemuck::cast_slice` requires `T: Pod`. All your numeric types
are. `bool` is not (Rust treats `bool` as a 1-byte type whose only valid
bit-patterns are 0 and 1, so casting `&[bool]` to `&[u8]` *is* sound but
needs `bytemuck::cast_slice` not to balk — actually `bool` doesn't impl
`Pod` so you'd handle it specially or use a bit-packed bool path).

### 2.2 `from_le_bytes` per element on read — same disease

In [column_reader.rs:74-78](src/column_reader.rs#L74-L78):
```rust
for _ in 0..count {
    let v = T::from_le_bytes(&block[cursor..cursor + elem_size]);
    out.push(v);
    cursor += elem_size;
}
```

Each iteration does:
- A bounds-checked slice
- `try_into()` → `[u8; N]` (small copy)
- `T::from_le_bytes` (a transmute on LE)
- `Vec::push` — possibly reallocating if `out` outgrows capacity

You allocated `out` with the right capacity, so `push` is fine. But the
per-element loop still costs you a copy and the loop overhead.

**Bulk fix:** the decompressed `block` is already `&[u8]`. The granule
range `&block[start..end]` is too. On LE, that's bit-identical to a
`&[T]` of length `(end-start)/sizeof(T)`. So:
```rust
let typed: &[T] = bytemuck::cast_slice(&block[start..end]);
out.extend_from_slice(typed);  // one memcpy
```
One bounds check + one memcpy + one allocation, instead of `count` of
each. For a granule of 1024 `i64`s: ~1 µs vs ~10–20 µs.

**Even better for scans (no copy at all):** return `&[T]` borrowing from
the cached block. The caller iterates without the granule ever materializing
into a `Vec<T>`. This is the Apache Arrow model and the reason aggregation
engines are fast.

### 2.3 Why this is the dominant issue

Storage engines spend their CPU on three things: I/O, compression, and
moving bytes between buffers. You can't do much about (1) and (2) — they
are what they are. But (3) is where naive code burns 5-10× more cycles than
necessary, because every "small" allocation or copy multiplies by row count.

A useful mental model: **for batch operations, do `O(1)` work per
*granule*, not per *row***. The current `to_le_bytes_vec` /
`from_le_bytes` loops violate this. Every other "make it faster" trick
(SIMD, prefetch, parallelism) is downstream of fixing this.

---

## 3. Performance — String Encoding

You asked me to re-explain point 4. Here's the long version.

### 3.1 Current format

For each string in a granule, the writer emits:
```
[u32 length][utf8 bytes][u32 length][utf8 bytes]...
```

To read string #5, the reader has to walk strings 0–4: read length, skip
that many bytes, read length, skip that many bytes, … This is **O(N) per
random access** within a granule.

To return the strings, the reader does:
```rust
let s = std::str::from_utf8(&block[cursor..cursor+len])?.to_owned();
out.push(s);
```
`to_owned()` allocates a new `String` (heap), memcpys the UTF-8 bytes into
it. One allocation per string. 1024 allocations per granule.

### 3.2 Parquet/Arrow-style format: split arrays

Store the granule as **two parallel arrays**:
```
offsets: [u32; N+1]   = [0, 5, 11, 11, 18, ...]   ← byte offsets into values
values:  [u8; total]  = "applebanana...kiwi"      ← all utf-8 concatenated
```
String `i` is `&values[offsets[i] .. offsets[i+1]]`. No length prefixes
embedded in the data; lengths are derived from offset deltas.

### 3.3 Why this is better, with intuition

**Random access is O(1).** Two array lookups, one slice. Today: walk from
start. For batch scans this matters because you can skip strings that fail
a predicate without reading them.

**Cache behaviour.** Offsets are tiny (4 bytes each) and contiguous — they
fit in L1. The values array is one big sequential blob. Modern CPUs love
sequential reads. The current interleaved layout makes the prefetcher work
harder.

**Zero-copy reads.** The reader can return `Vec<&str>` borrowing into the
cached block — no per-string `String` allocation. For 1024 strings, that's
1024 mallocs avoided per granule.

**Smaller in many cases.** If your average string is short (say 8 bytes),
the length prefix is 50% overhead today (4 of every 12 bytes). Offsets
amortize: `4 * (N+1)` total instead of `4 * N` inline. Same size today,
but offsets compress *brilliantly* (deltas between offsets are small
positive ints — perfect for FOR/bit-packing later).

**Plays nicely with dictionary encoding later.** When you swap raw values
for dict-codes, the offsets array stays the same conceptually (now offsets
into a code array). Today's interleaved format doesn't generalize.

### 3.4 Worked size example

100 strings averaging 16 bytes each:
- **Today:** `100 * (4 + 16) = 2000` bytes; reader does 100 length parses.
- **Split:** `(100+1)*4 + 100*16 = 2004` bytes; reader does 0 length
  parses, returns 100 `&str` slices into the same buffer.

Same bytes on disk, but the read path is dramatically simpler and
allocation-free. And once you compress the offsets (delta + bit-pack), the
split layout pulls ahead on size too.

---

## 4. Performance — Reader Path

### 4.1 Single-block cache (point 5 originally)

Your reader keeps one decompressed block in memory and evicts on the next
block. Worried me earlier because random access across blocks is pessimal —
but you said batch scans only. **For sequential scans this is optimal.**
You read every granule in order, the cache is hit for every granule in the
same block, and you decompress each block exactly once. Skip.

The one tweak: when you do start scanning multiple columns in parallel, you
want the cache *per column reader*, not shared. You already have this.

### 4.2 `Vec<T>` allocation per `read_granule` (point 6)

Every call to `read_granule` allocates a new `Vec<T>` and returns it by
value. For a scan of N granules, that's N allocations and N drops. Most of
those `Vec`s have the same capacity (1024 elements) — you're hitting the
allocator with the exact same `malloc(8192)` over and over.

**Fix:** scan API that reuses a buffer.
```rust
fn read_granule_into(&mut self, idx: usize, out: &mut Vec<T>) -> io::Result<()>;
```
Caller allocates one `Vec<T>` with `with_capacity(GRANULE_SIZE)`, calls
`out.clear(); reader.read_granule_into(i, &mut out);` per granule. Zero
allocations after the first granule.

Worked numbers, scanning 1000 granules of i64:
- Today: 1000 mallocs of 8 KB = ~50 µs of allocator overhead, plus
  fragmentation pressure.
- Reused buffer: 1 malloc of 8 KB = ~50 ns total. **Three orders of
  magnitude.**

For string columns the savings are bigger because each `String` inside the
`Vec<String>` is its own allocation.

**Even better — borrowed scan:** `fn next_granule(&mut self) -> io::Result
<&[T]>` returns a slice into the cache. Zero copy, zero allocation. The
caller's loop body operates on `&[T]` and never owns the data. This is what
column engines do for aggregations — your `SUM(column)` kernel is just
`slice.iter().sum()` over each granule.

### 4.3 `pread` / positioned I/O (point 7)

Today's reader does:
```rust
self.bin.seek(SeekFrom::Start(mark.block_offset))?;
self.bin.read_exact(&mut compressed)?;
```
That's two syscalls: `lseek` + `read`. `lseek` mutates the file's
"current position" — which is **shared state** on the file descriptor.

`pread` (`FileExt::read_exact_at` on Unix) is one syscall and takes the
offset as an argument. Doesn't touch the cursor.

**Why it matters for tinyOLAP:**
1. **One syscall is faster than two.** ~500 ns saved per granule read.
   Not huge per-granule, but adds up at scale.
2. **No shared mutable cursor → can read from multiple threads with `&File`,
   not `&mut File`.** This is the bigger win. To scan two columns in
   parallel today you need two `File` opens or a `Mutex<File>`. With
   `pread` you can `Arc<File>` and have N threads issue reads concurrently.
   The kernel handles the seeking internally per request.

Worked example — parallel column scan:
```text
seek+read:  Mutex<File> → threads serialize on the mutex → no parallelism
pread:      Arc<File>   → all threads issue independent positioned reads
                          → kernel + SSD do them concurrently
```

### 4.4 `mmap` (point 8)

`mmap` maps the file's contents into your process's virtual address space.
Reading the file becomes pointer arithmetic into a `&[u8]` — the kernel
handles paging blocks in on demand.

**Why it's compelling for a columnar reader:**
- **Zero-copy.** Today you `read_exact` into a `Vec<u8>`, decompress into
  another `Vec<u8>`. With mmap, the compressed bytes are *already* a
  `&[u8]` — you skip the read-into-buffer copy.
- **OS-managed cache.** The page cache is shared across processes and
  managed by the kernel's LRU. Your single-block cache is replaced by
  "whatever the kernel decided to keep resident".
- **Trivial parallelism.** Multiple threads dereference the same mmap
  region. No locks, no `pread`.
- **Marks file is perfect for mmap.** Your `.mrk` is a packed array of
  `Mark` structs. With `#[repr(C)]` + `bytemuck::Pod`, mmapping it gives
  you `&[Mark]` for free — zero parse cost, zero allocation. Today you
  `read_to_end` into a `Vec<u8>` and parse 24 bytes at a time.

**Caveats:**
- mmap I/O errors become `SIGBUS`, not `Result::Err`. A truncated file
  will crash the process when you read into the missing region. This is
  the main reason production systems sometimes prefer `pread` despite the
  speed.
- Memory accounting is weirder — `top` shows mapped-but-not-resident pages
  oddly. Operationally annoying but not a correctness problem.
- Random writes through mmap are a footgun. Read-only mmap (which is your
  use case) is safe and simple.

**Recommendation for tinyOLAP:** mmap the `.mrk` files (huge win, zero
risk). For `.bin`, start with `pread` (simpler, errors are recoverable).
Move to mmap later if profiling says decompression-target allocation is a
hotspot.

### 4.5 Per-column pipelining (point 9)

`rayon::par_iter` splits the columns across worker threads. Each worker
runs `write_one_column` start-to-finish. This is fine when columns are
similar in size. It breaks down when one column is much bigger (e.g. a
string column with long values, vs. an i32 id column).

Worked example, 8 columns, one is 10× bigger:
```text
par_iter (today):
  thread 0:  [================================================] big col
  thread 1:  [====]                                              col 2
  thread 2:  [====]                                              col 3
  thread 3:  [====]                                              col 4
             ^—— 7 threads idle while col 0 finishes
  total wall time = max column time

Pipelined:
  producer thread: read input rows → fan out per-column channels
  worker per column: pull from channel, encode, compress, write
  Each column makes progress independently; small columns finish early
  and free their thread.
  total wall time ≈ max column time still — BUT...
```

For your workload (1 MB inserts, ~100k rows max), a single insert is
small enough that the rayon parallelism is already overkill. Pipelining
matters when:
- Inserts are streaming (the spec hints at batching being a future thing)
- One column dominates (very wide strings)
- You want to overlap encode+compress+write with the next batch's
  encode+compress

**Recommendation:** keep rayon for now. Revisit when you have larger
batches or streaming inserts.

### 4.6 mmap for `.mrk` (point 10, expanded)

Mentioned above — worth its own line. The mark file is a perfectly packed
array of fixed-size records. On read today you do:
```rust
let mut buf = Vec::new();
self.file.read_to_end(&mut buf)?;
buf.chunks_exact(24).map(Mark::from_bytes).collect()
```
For 100 marks: 1 read syscall, 1 alloc of 2.4 KB, 100 parses, 1 alloc of
the result `Vec<Mark>`. Total ~5 µs.

With `repr(C)` + mmap:
```rust
let mmap = Mmap::map(&file)?;
let marks: &[Mark] = bytemuck::cast_slice(&mmap);
```
Zero allocations, zero parses. ~50 ns. The reader holds the `Mmap`
alongside the `&[Mark]` slice (lifetime-tied).

Side note: you currently allocate 24 bytes per mark but only use 16. Drop
to `[u8; 16]` (or use the extra 8 bytes for something — see §6).

---

## 5. API & Structure

### 5.1 `IDataType::size_of()` — redundant

`std::mem::size_of::<T>()` is a const intrinsic. The reader already uses
it on line 65. Drop the method from the trait. One less thing to keep
consistent across impls.

### 5.2 `bool` storage — 8× compression for free

Today: 1 byte per bool. A bitmap (1 bit per bool, packed 8-to-a-byte) is
8× smaller and compresses better afterward. This is encoding territory
which you flagged as TBD — noting it here for completeness.

### 5.3 Nullability — design now or pay later

No column today supports `NULL`. Real OLAP queries care about nullability
(`COUNT(*)` vs `COUNT(col)` differ when there are nulls). Arrow's model:
each column has an optional **validity bitmap**, 1 bit per row, 1=valid
0=null. Stored as a sibling file (`.null`) or a sidecar inside the block.

Why design now: retrofitting nullability touches every reader, every
writer, every encoding. Adding the bitmap from the start costs a few
hundred lines; adding it later is a refactor.

### 5.4 `ColumnChunk` enum + 12-arm matches

`table_writer.rs` and `table_reader.rs` both have a 12-arm match on
`DataType`. Adding a new dtype means edits in 4+ places. A trait-object
approach (`Box<dyn ColumnHandler>` keyed on dtype, with `write_chunk` and
`read_granule` methods) collapses this to one-file changes.

This is mostly a "growing pains" call — fine while there are 12 dtypes;
gets ugly at 30. For now, leave it.

### 5.5 `PhantomData<T>` in `ColumnWriter`

`ColumnWriter<T>` parameterizes the struct only because `write_chunk`
needs `T`. You could make the struct non-generic and put the type
parameter on `write_chunk` only. The benefit is downstream code can hold
a `Vec<ColumnWriter>` without enum dispatch. Minor; defer.

### 5.6 `part.meta` — the most important missing piece

A part today is "whatever files are in the directory". You can't ask:
- How many rows?
- What's the min/max of column X? (predicate pushdown)
- What dtype was column X when this part was written? (schema evolution)
- Is the part complete? (crash recovery)
- What was the encoding? (forward compat)

A `part.meta` file written **last** (after all `.bin` and `.mrk` are
fsynced) and itself fsynced before the rename solves all of this:
```text
part_00042/
  user_id.bin
  user_id.mrk
  email.bin
  email.mrk
  part.meta   ← written and fsynced last
```
Crash recovery: any part dir without a valid `part.meta` is incomplete and
gets deleted on startup. **This is how you actually achieve atomic parts.**
The rename trick alone is necessary but not sufficient — without a sentinel
file, a half-written part dir that happens to contain the right filenames
would look complete.

Contents (rough):
- magic + version
- row count
- per-column: name, dtype, null count, min, max, encoding id, byte size,
  block count, granule count, optional checksum
- writer timestamp / writer id

This unlocks the entire query optimizer: "this part's `user_id` min/max is
[100, 200], the query asks `WHERE user_id = 5`, skip the part entirely."
For a scan workload, that's the difference between a 10 ms query and a 10 s
query.

### 5.7 Per-block checksums

Today, a single bit-flip in a `.bin` file causes either:
- LZ4 returns garbage (data corruption silently goes through), or
- LZ4 panics on a bad header (process dies)

Store an `xxhash3_64` (3-5 ns/byte, faster than LZ4) of each compressed
block alongside its length:
```text
[u32 compressed_len][u64 hash][compressed bytes]
```
On read, hash the bytes and compare. Mismatch → return an error, log,
quarantine the part. This is cheap insurance. One bad SSD sector
otherwise corrupts a whole table silently.

### 5.8 Magic byte + format version

First 8 bytes of every `.bin` and `.mrk`: `b"TINYOLAP"` + a `u8` version.
A reader that opens a v2 file written by v1 code (or vice versa) should
fail loudly, not parse garbage. Six bytes of disk now saves a debugging
weekend later.

---

## 6. Smaller Cleanups

- `mark.rs:12` claims `[u8; 24]` but only fills 16. Either drop to `[u8;
  16]` or use the extra 8 bytes for something useful: rows-in-granule is
  the obvious candidate (lets the reader compute `count` without diffing
  with the next mark).
- `string_column_reader.rs:73` `.to_owned()` clones every string out of
  the cached block. See §3 — a borrowed-scan API removes this entirely.
- `string_column_writer.rs` uses verbose `OpenOptions::new()...` where
  `File::create` would do. Numeric writer already does the short form.
- `block_buf: Vec::with_capacity(BLOCK_BUFFER_SIZE * 2)`: the `* 2` is a
  fudge factor for granule overflow. Either size exactly and seal at
  threshold, or document why 2× is the safe over-estimate.
- `column_writer.rs:92` resets `rows_in_current_granule = 0` then calls
  `flush_block`, not `seal_granule`. The trailing partial granule never
  gets a mark. The reader compensates because it uses
  "next-mark-or-end-of-block" for the granule's end. **This works but is
  asymmetric** with the string writer, which seals trailing partials. Pick
  one convention and stick to it; document why in a comment.

---

## 7. Suggested Priority Order

Given your constraints (atomic-per-insert, batch scans, ~1 MB inserts):

1. **fsync correctness** (§1.1) — non-negotiable.
2. **Kill `to_le_bytes_vec` allocation + `from_le_bytes` per-element**
   (§2.1, §2.2) — biggest perf win, smallest diff.
3. **`part.meta` + magic/version** (§5.6, §5.8) — unlocks crash recovery
   and future planner work.
4. **Per-block checksum** (§5.7) — cheap insurance.
5. **String split-array layout** (§3) — needed before any string-side
   optimization makes sense.
6. **`pread` on `.bin`, mmap on `.mrk`** (§4.3, §4.6) — cleaner concurrency
   story, modest perf.
7. **Borrowed-scan reader API** (§4.2) — sets up the aggregation engine.
8. **Nullability bitmap** (§5.3) — design now, retrofit is painful.

Everything below this line (encoding schemes, pipelined writers, trait-
object dispatch) is deferrable and you've already flagged most of it.
