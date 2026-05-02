//! Delta encoding for integer primitives.
//!
//! Format: [first_value: T LE][delta_1: T LE]...
//! Each delta is `value[i].wrapping_sub(value[i-1])`. Wrapping arithmetic
//! makes the round-trip correct even when subtraction would overflow.

use crate::encoding::Primitive;

pub fn encode<T: Primitive>(src: &[T], out: &mut Vec<u8>) {
    if src.is_empty() {
        return;
    }

    src[0].encode_le(out);

    let mut prev = src[0];
    for &v in &src[1..] {
        let delta = v.wrapping_sub(prev);
        delta.encode_le(out);
        prev = v;
    }
}

pub fn decode<T: Primitive>(src: &[u8], out: &mut Vec<T>) {
    if src.is_empty() {
        return;
    }

    let mut chunks = src.chunks_exact(T::WIDTH);

    let first = T::decode_le(chunks.next().unwrap());
    out.push(first);

    let mut prev = first;
    for chunk in chunks {
        let delta = T::decode_le(chunk);
        let v = prev.wrapping_add(delta);
        out.push(v);
        prev = v;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn roundtrip_i32_basic() {
        let xs = vec![100_i32, 105, 103, 200, 199];
        let mut bytes = Vec::new();
        encode(&xs, &mut bytes);
        let mut out: Vec<i32> = Vec::new();
        decode(&bytes, &mut out);
        assert_eq!(xs, out);
    }

    #[test]
    fn roundtrip_empty() {
        let xs: Vec<i32> = vec![];
        let mut bytes = Vec::new();
        encode(&xs, &mut bytes);
        assert!(bytes.is_empty());
        let mut out: Vec<i32> = Vec::new();
        decode(&bytes, &mut out);
        assert_eq!(xs, out);
    }

    #[test]
    fn roundtrip_single_i32() {
        let xs = vec![42_i32];
        let mut bytes = Vec::new();
        encode(&xs, &mut bytes);
        assert_eq!(bytes.len(), 4); // just the first value, no delta yet
        let mut out: Vec<i32> = Vec::new();
        decode(&bytes, &mut out);
        assert_eq!(xs, out);
    }

    #[test]
    fn roundtrip_i32_overflow() {
        // i32::MAX → i32::MIN forces a delta that doesn't fit in i32.
        // The wrapping_sub/wrapping_add pair must cancel out.
        let xs = vec![i32::MAX, i32::MIN, 0, i32::MAX];
        let mut bytes = Vec::new();
        encode(&xs, &mut bytes);
        let mut out: Vec<i32> = Vec::new();
        decode(&bytes, &mut out);
        assert_eq!(xs, out);
    }

    #[test]
    fn roundtrip_i64_overflow() {
        let xs = vec![i64::MAX, i64::MIN, 0, i64::MAX];
        let mut bytes = Vec::new();
        encode(&xs, &mut bytes);
        let mut out: Vec<i64> = Vec::new();
        decode(&bytes, &mut out);
        assert_eq!(xs, out);
    }

    #[test]
    fn roundtrip_u32_basic() {
        let xs = vec![10_u32, 12, 15, 3, u32::MAX];
        let mut bytes = Vec::new();
        encode(&xs, &mut bytes);
        let mut out: Vec<u32> = Vec::new();
        decode(&bytes, &mut out);
        assert_eq!(xs, out);
    }

    #[test]
    fn format_u32() {
        let xs = vec![10_u32, 12, 15];
        let mut bytes = Vec::new();
        encode(&xs, &mut bytes);

        let expected = vec![
            10, 0, 0, 0,
            2, 0, 0, 0,
            3, 0, 0, 0,
        ];

        assert_eq!(bytes, expected);
    }
}
