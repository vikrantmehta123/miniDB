//! Format: [count: u16 LE][value: T LE] pairs.
//!
//! `count` is u16 because at the current 32 KiB raw block buffer, the most
//! values a block can hold is 32 KiB / 1 byte = 32768 (for u8/i8 columns),
//! well under u16::MAX. If BLOCK_BUFFER_SIZE grows past ~64 KiB raw,
//! revisit: either widen count to u32, or rely on the in-encoder split
//! at u16::MAX (already implemented).

use crate::encoding::{Primitive, EncodingError};

pub fn encode<T: Primitive>(src: &[T], out: &mut Vec<u8>) {
    if src.is_empty() {
        return;
    }

    let mut prev = src[0];
    let mut count: u16 = 1;

    for &v in &src[1..] {
        if v == prev && count < u16::MAX {
            count += 1;
        } else {
            out.extend_from_slice(&count.to_le_bytes());
            prev.encode_le(out);
            prev = v;
            count = 1;
        }
    }

    // Flush the final run — the loop never emits its last pair.
    out.extend_from_slice(&count.to_le_bytes());
    prev.encode_le(out);
}

pub fn decode<T: Primitive>(src: &[u8], out: &mut Vec<T>) -> Result<(), EncodingError> {
    let pair_size = 2 + T::WIDTH;
    let mut i = 0;

    while i < src.len() {
        if src.len() - i < pair_size {
            return Err(EncodingError::Truncated);
        }

        let count = u16::from_le_bytes(src[i..i + 2].try_into().unwrap());
        let value = T::decode_le(&src[i + 2..i + 2 + T::WIDTH]);

        for _ in 0..count {
            out.push(value);
        }

        i += pair_size;
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn roundtrip_basic() {
        let xs = vec![7_i32, 7, 7, 3, 3, 9, 9, 9, 9];
        let mut bytes = Vec::new();
        encode(&xs, &mut bytes);
        let mut out: Vec<i32> = Vec::new();
        decode(&bytes, &mut out).unwrap();
        assert_eq!(xs, out);
    }

    #[test]
    fn roundtrip_empty() {
        let xs: Vec<i32> = vec![];
        let mut bytes = Vec::new();
        encode(&xs, &mut bytes);
        assert!(bytes.is_empty());
        let mut out: Vec<i32> = Vec::new();
        decode(&bytes, &mut out).unwrap();
        assert_eq!(xs, out);
    }

    #[test]
    fn roundtrip_single_value() {
        // exercises the "loop body never runs, flush still fires" path
        let xs = vec![42_i32];
        let mut bytes = Vec::new();
        encode(&xs, &mut bytes);
        assert_eq!(bytes.len(), 6); // one pair: 2 + 4
        let mut out: Vec<i32> = Vec::new();
        decode(&bytes, &mut out).unwrap();
        assert_eq!(xs, out);
    }

    #[test]
    fn roundtrip_no_runs() {
        // worst case: every value differs from its neighbor
        let xs = vec![1_i32, 2, 3, 4, 5];
        let mut bytes = Vec::new();
        encode(&xs, &mut bytes);
        assert_eq!(bytes.len(), 5 * 6); // 5 pairs of 6 bytes
        let mut out: Vec<i32> = Vec::new();
        decode(&bytes, &mut out).unwrap();
        assert_eq!(xs, out);
    }

    #[test]
    fn roundtrip_one_giant_run() {
        let xs = vec![42_i32; 1000];
        let mut bytes = Vec::new();
        encode(&xs, &mut bytes);
        assert_eq!(bytes.len(), 6); // single pair
        let mut out: Vec<i32> = Vec::new();
        decode(&bytes, &mut out).unwrap();
        assert_eq!(xs, out);
    }

    #[test]
    fn run_split_across_u16_max() {
        // 65536 identical values must split into two pairs (65535 + 1)
        let xs = vec![5_i32; 65536];
        let mut bytes = Vec::new();
        encode(&xs, &mut bytes);
        assert_eq!(bytes.len(), 2 * 6);
        let mut out: Vec<i32> = Vec::new();
        decode(&bytes, &mut out).unwrap();
        assert_eq!(xs, out);
    }

    #[test]
    fn truncated_input_rejected() {
        // 1 byte — not enough for even the count
        let bytes = vec![0_u8];
        let mut out: Vec<i32> = Vec::new();
        assert_eq!(decode(&bytes, &mut out), Err(EncodingError::Truncated));
    }

    #[test]
    fn format_lock() {
        let xs = vec![7_u32, 7, 7, 3];
        let mut bytes = Vec::new();
        encode(&xs, &mut bytes);

        let expected = vec![
            3, 0,             // count = 3 (u16 LE)
            7, 0, 0, 0,       // value = 7 (u32 LE)
            1, 0,             // count = 1
            3, 0, 0, 0,       // value = 3
        ];
        assert_eq!(bytes, expected);
    }
}
