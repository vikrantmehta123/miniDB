//! Plain encoding: values written as raw little-endian bytes, no transformation.

use crate::encoding::{Primitive, EncodingError};

pub fn encode<T: Primitive>(src: &[T], out: &mut Vec<u8>) {
    for &v in src {
        v.encode_le(out);
    }
}

pub fn decode<T: Primitive>(src: &[u8], out: &mut Vec<T>) -> Result<(), EncodingError> {
    // Validate the length of the source is a multiple of T's size
    if src.len() % T::WIDTH != 0 {
       return Err(EncodingError::Truncated);
    }
    
    for chunk in src.chunks_exact(T::WIDTH) {
        out.push(T::decode_le(chunk));
    }
    Ok(())
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
    fn roundtrip_u64_basic() {
        let xs = vec![0_u64, 1, u64::MAX, 42, u64::MAX / 2];
        let mut bytes = Vec::new();
        encode(&xs, &mut bytes);
        assert_eq!(bytes.len(), xs.len() * std::mem::size_of::<u64>());
        let mut out: Vec<u64> = Vec::new();
        decode(&bytes, &mut out).unwrap();
        assert_eq!(xs, out);
    }

    #[test]
    fn roundtrip_i8_extremes() {
        let xs = vec![i8::MIN, -1, 0, 1, i8::MAX];
        let mut bytes = Vec::new();
        encode(&xs, &mut bytes);
        let mut out: Vec<i8> = Vec::new();
        decode(&bytes, &mut out).unwrap();
        assert_eq!(xs, out);
    }

    #[test]
    fn format_u32() {
        let xs = vec![10_u32, 12, 15];
        let mut bytes = Vec::new();
        encode(&xs, &mut bytes);

        let expected = vec![
            10, 0, 0, 0,
            12, 0, 0, 0,
            15, 0, 0, 0,
        ];

        assert_eq!(bytes, expected);
    }
}
