use std::fs::File;
use std::io::{self, BufWriter, Write};
use std::path::Path;

use crate::config::{BLOCK_BUFFER_SIZE, GRANULE_SIZE};
use crate::column_writer::ColumnStats;
use crate::encoding::StringCodec;
use crate::mark::{Mark, MarkWriter};

/// Write one string column to disk for a single part.
///
/// Strings are variable-length so they can't use the stride-based `Codec` path.
/// `StringCodec` operates on `&[String]` directly.
///
/// Block framing: [u8 codec_tag][u32 LE compressed_len][compressed bytes]
///
/// `decompressed_offset` in marks is a byte offset into the *plain-decoded*
/// stream (u32 length prefix + utf8 bytes per string). The reader decodes with
/// `codec.decode` first, then uses this offset to find the granule start —
/// consistent regardless of which StringCodec was used.
pub fn write_string_column(
    part_dir: &Path,
    col_name: &str,
    values: &[String],
    codec: StringCodec,
) -> io::Result<ColumnStats> {
    let bin_path = part_dir.join(format!("{col_name}.bin"));
    let mrk_path = part_dir.join(format!("{col_name}.mrk"));

    let mut bin = BufWriter::new(File::create(bin_path)?);
    let mut marks = MarkWriter::create(&mrk_path)?;

    let mut block_strings: Vec<String> = Vec::new();
    let mut pending_marks: Vec<Mark> = Vec::new();

    let mut rows_in_current_granule: usize = 0;
    // Tracks how many plain bytes are in the current block so we know when to
    // flush. Plain size = 4 (u32 length prefix) + utf8 byte count per string.
    let mut plain_bytes_in_block: usize = 0;
    let mut total_rows: u64 = 0;
    let mut bin_bytes: u64 = 0;

    for s in values {
        let plain_size = 4 + s.len();

        // Size cap: if this string would push the block past BLOCK_BUFFER_SIZE,
        // close the current granule and flush now. Never split a string across blocks.
        // The mark for this granule was already pushed when it started below.
        if rows_in_current_granule > 0
            && plain_bytes_in_block + plain_size > BLOCK_BUFFER_SIZE
        {
            rows_in_current_granule = 0;
            flush_block(
                codec,
                &mut block_strings,
                &mut pending_marks,
                &mut bin,
                &mut marks,
                &mut bin_bytes,
                &mut plain_bytes_in_block,
            )?;
        }

        // First row of a new granule: record where it starts in the plain byte
        // stream. block_offset is patched at flush_block once we know it.
        if rows_in_current_granule == 0 {
            pending_marks.push(Mark {
                block_offset: 0,
                decompressed_offset: plain_bytes_in_block as u64,
            });
        }

        block_strings.push(s.clone());
        rows_in_current_granule += 1;
        plain_bytes_in_block += plain_size;
        total_rows += 1;

        // Row cap: granules are also bounded by GRANULE_SIZE regardless of size.
        if rows_in_current_granule == GRANULE_SIZE {
            rows_in_current_granule = 0;
            if plain_bytes_in_block >= BLOCK_BUFFER_SIZE {
                flush_block(
                    codec,
                    &mut block_strings,
                    &mut pending_marks,
                    &mut bin,
                    &mut marks,
                    &mut bin_bytes,
                    &mut plain_bytes_in_block,
                )?;
            }
        }
    }

    // Flush the tail block — may be a partial granule, its mark was already pushed.
    flush_block(
        codec,
        &mut block_strings,
        &mut pending_marks,
        &mut bin,
        &mut marks,
        &mut bin_bytes,
        &mut plain_bytes_in_block,
    )?;

    bin.flush()?;
    bin.get_ref().sync_all()?;
    marks.flush()?;

    Ok(ColumnStats { rows: total_rows, bin_bytes })
}

/// Encode, compress and write the current block. Patches pending marks with
/// the real block_offset. No-op on empty buffer.
fn flush_block(
    codec: StringCodec,
    block_strings: &mut Vec<String>,
    pending_marks: &mut Vec<Mark>,
    bin: &mut BufWriter<File>,
    marks: &mut MarkWriter,
    bin_bytes: &mut u64,
    plain_bytes_in_block: &mut usize,
) -> io::Result<()> {
    if block_strings.is_empty() {
        return Ok(());
    }

    // Encode with StringCodec, then lz4 compress.
    let mut encoded: Vec<u8> = Vec::new();
    codec.encode(block_strings, &mut encoded);
    let compressed = lz4_flex::compress_prepend_size(&encoded);

    let block_offset = *bin_bytes;
    bin.write_all(&[codec.tag()])?;
    bin.write_all(&(compressed.len() as u32).to_le_bytes())?;
    bin.write_all(&compressed)?;
    *bin_bytes += 1 + 4 + compressed.len() as u64;

    for mut mark in pending_marks.drain(..) {
        mark.block_offset = block_offset;
        marks.write(&mark);
    }

    block_strings.clear();
    *plain_bytes_in_block = 0;
    Ok(())
}
