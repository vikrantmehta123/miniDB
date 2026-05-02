//! Delta encoding for i32. 
//! 
//! Format: [first_value: i32 LE][delta_1: i32 LE]...
//! Each delta is `value[i].wrapping_sub(value[i-1])`. Wrapping arithmetic
//! makes the round-trip correct even when subtraction would overflow.


pub fn encode(src: &[i32], out: &mut Vec<u8>) {
    if src.is_empty() {
        return;
    }

    out.extend_from_slice(&src[0].to_le_bytes());
    let mut prev = src[0];
    for &v in &src[1..] {
        let delta = v.wrapping_sub(prev);
        out.extend_from_slice(&delta.to_le_bytes());
        prev = v;
    }
}

pub fn decode(src: &[u8], out: &mut Vec<i32>) {
    if src.is_empty() {
        return;
    }
    let mut chunks = src.chunks_exact(4);
    let first = i32::from_le_bytes(chunks.next().unwrap().try_into().unwrap());
    out.push(first);
    let mut prev = first;
    for chunk in chunks {
        let delta = i32::from_le_bytes(chunk.try_into().unwrap());
        let v = prev.wrapping_add(delta);
        out.push(v);
        prev = v;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn roundtrip_basic() {
        let xs = vec![100, 105, 103, 200, 199];
        let mut bytes = Vec::new();
        encode(&xs, &mut bytes);
        let mut out = Vec::new();
        decode(&bytes, &mut out);
        assert_eq!(xs, out);
    }

    #[test]
    fn roundtrip_empty() {
        let xs: Vec<i32> = vec![];
        let mut bytes = Vec::new();
        encode(&xs, &mut bytes);
        assert!(bytes.is_empty());
        let mut out = Vec::new();
        decode(&bytes, &mut out);
        assert_eq!(xs, out);
    }

    #[test]
    fn roundtrip_single() {
        let xs = vec![42_i32];
        let mut bytes = Vec::new();
        encode(&xs, &mut bytes);
        assert_eq!(bytes.len(), 4); // just the first value, no delta yet
        let mut out = Vec::new();
        decode(&bytes, &mut out);
        assert_eq!(xs, out);
    }

    #[test]
    fn roundtrip_overflow() {
        // i32::MAX → i32::MIN forces a delta that doesn't fit in i32.
        // The wrapping_sub/wrapping_add pair must cancel out.
        let xs = vec![i32::MAX, i32::MIN, 0, i32::MAX];
        let mut bytes = Vec::new();
        encode(&xs, &mut bytes);
        let mut out = Vec::new();
        decode(&bytes, &mut out);
        assert_eq!(xs, out);
    }
}
