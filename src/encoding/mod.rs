//! Encoding library for column data.
//!
//! Codecs transform typed value slices into byte streams (and back) before
//! lz4 compression. Each codec is self-describing: encoded output carries
//! enough header for `decode` to reverse it without external metadata.

pub mod delta;
pub mod plain;
pub mod rle;

mod sealed {
    pub trait Sealed {}
}

pub trait Primitive: Copy + PartialEq + sealed::Sealed {
    const WIDTH: usize;

    fn encode_le(self, out: &mut Vec<u8>);
    fn decode_le(bytes: &[u8]) -> Self;
    fn wrapping_sub(self, rhs: Self) -> Self;
    fn wrapping_add(self, rhs: Self) -> Self;
}

impl sealed::Sealed for i8 {}

impl Primitive for i8 {
    const WIDTH: usize = std::mem::size_of::<Self>();

    fn encode_le(self, out: &mut Vec<u8>) {
        out.extend_from_slice(&self.to_le_bytes());
    }
    fn decode_le(bytes: &[u8]) -> Self {
        Self::from_le_bytes(bytes.try_into().unwrap())
    }
    fn wrapping_sub(self, rhs: Self) -> Self {
        self.wrapping_sub(rhs)
    }
    fn wrapping_add(self, rhs: Self) -> Self {
        self.wrapping_add(rhs)
    }
}

impl sealed::Sealed for i16 {}
impl Primitive for i16 {
    const WIDTH: usize = std::mem::size_of::<Self>();

    fn encode_le(self, out: &mut Vec<u8>) {
        out.extend_from_slice(&self.to_le_bytes());
    }
    fn decode_le(bytes: &[u8]) -> Self {
        Self::from_le_bytes(bytes.try_into().unwrap())
    }
    fn wrapping_sub(self, rhs: Self) -> Self {
        self.wrapping_sub(rhs)
    }
    fn wrapping_add(self, rhs: Self) -> Self {
        self.wrapping_add(rhs)
    }
}

impl sealed::Sealed for i32 {}
impl Primitive for i32 {
    const WIDTH: usize = std::mem::size_of::<Self>();

    fn encode_le(self, out: &mut Vec<u8>) {
        out.extend_from_slice(&self.to_le_bytes());
    }
    fn decode_le(bytes: &[u8]) -> Self {
        Self::from_le_bytes(bytes.try_into().unwrap())
    }
    fn wrapping_sub(self, rhs: Self) -> Self {
        self.wrapping_sub(rhs)
    }
    fn wrapping_add(self, rhs: Self) -> Self {
        self.wrapping_add(rhs)
    }
}

impl sealed::Sealed for i64 {}
impl Primitive for i64 {
    const WIDTH: usize = std::mem::size_of::<Self>();

    fn encode_le(self, out: &mut Vec<u8>) {
        out.extend_from_slice(&self.to_le_bytes());
    }
    fn decode_le(bytes: &[u8]) -> Self {
        Self::from_le_bytes(bytes.try_into().unwrap())
    }
    fn wrapping_sub(self, rhs: Self) -> Self {
        self.wrapping_sub(rhs)
    }
    fn wrapping_add(self, rhs: Self) -> Self {
        self.wrapping_add(rhs)
    }
}

impl sealed::Sealed for u8 {}
impl Primitive for u8 {
    const WIDTH: usize = std::mem::size_of::<Self>();

    fn encode_le(self, out: &mut Vec<u8>) {
        out.extend_from_slice(&self.to_le_bytes());
    }
    fn decode_le(bytes: &[u8]) -> Self {
        Self::from_le_bytes(bytes.try_into().unwrap())
    }
    fn wrapping_sub(self, rhs: Self) -> Self {
        self.wrapping_sub(rhs)
    }
    fn wrapping_add(self, rhs: Self) -> Self {
        self.wrapping_add(rhs)
    }
}

impl sealed::Sealed for u16 {}
impl Primitive for u16 {
    const WIDTH: usize = std::mem::size_of::<Self>();

    fn encode_le(self, out: &mut Vec<u8>) {
        out.extend_from_slice(&self.to_le_bytes());
    }
    fn decode_le(bytes: &[u8]) -> Self {
        Self::from_le_bytes(bytes.try_into().unwrap())
    }
    fn wrapping_sub(self, rhs: Self) -> Self {
        self.wrapping_sub(rhs)
    }
    fn wrapping_add(self, rhs: Self) -> Self {
        self.wrapping_add(rhs)
    }
}

impl sealed::Sealed for u32 {}
impl Primitive for u32 {
    const WIDTH: usize = std::mem::size_of::<Self>();

    fn encode_le(self, out: &mut Vec<u8>) {
        out.extend_from_slice(&self.to_le_bytes());
    }
    fn decode_le(bytes: &[u8]) -> Self {
        Self::from_le_bytes(bytes.try_into().unwrap())
    }
    fn wrapping_sub(self, rhs: Self) -> Self {
        self.wrapping_sub(rhs)
    }
    fn wrapping_add(self, rhs: Self) -> Self {
        self.wrapping_add(rhs)
    }
}

impl sealed::Sealed for u64 {}
impl Primitive for u64 {
    const WIDTH: usize = std::mem::size_of::<Self>();

    fn encode_le(self, out: &mut Vec<u8>) {
        out.extend_from_slice(&self.to_le_bytes());
    }
    fn decode_le(bytes: &[u8]) -> Self {
        Self::from_le_bytes(bytes.try_into().unwrap())
    }
    fn wrapping_sub(self, rhs: Self) -> Self {
        self.wrapping_sub(rhs)
    }
    fn wrapping_add(self, rhs: Self) -> Self {
        self.wrapping_add(rhs)
    }
}


/// Identifies which encoding scheme was used to produce a byte stream.
/// Stored as a single byte at the start of each compressed block.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Codec {
    Plain,
    Delta,
    RLE,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EncodingError {
    Truncated,
    BadHeader,
}

impl Codec {
    pub fn encode<T: Primitive>(self, src: &[T], out: &mut Vec<u8>) {
        match self {
            Codec::Plain => plain::encode(src, out),
            Codec::Delta => delta::encode(src, out),
            Codec::RLE   => rle::encode(src, out),
        }
    }

    pub fn decode<T: Primitive>(self, src: &[u8], out: &mut Vec<T>) -> Result<(), EncodingError> {
        match self {
            Codec::Plain => plain::decode(src, out),
            Codec::Delta => delta::decode(src, out),
            Codec::RLE   => rle::decode(src, out),
        }
    }

    pub fn tag(self) -> u8 {
        match self {
            Codec::Plain => 0,
            Codec::Delta => 1,
            Codec::RLE   => 2,
        }
    }

    pub fn from_tag(tag: u8) -> Result<Self, EncodingError> {
        match tag {
            0 => Ok(Codec::Plain),
            1 => Ok(Codec::Delta),
            2 => Ok(Codec::RLE),
            _ => Err(EncodingError::BadHeader),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn codec_roundtrip_via_enum() {
        let xs = vec![100_i32, 105, 103, 200, 199];

        for codec in [Codec::Plain, Codec::Delta] {
            let mut bytes = Vec::new();
            codec.encode(&xs, &mut bytes);
            let mut out: Vec<i32> = Vec::new();
            codec.decode(&bytes, &mut out).unwrap();
            assert_eq!(xs, out, "codec {:?} failed roundtrip", codec);
        }
    }

    #[test]
    fn tag_roundtrip() {
        for codec in [Codec::Plain, Codec::Delta] {
            assert_eq!(Codec::from_tag(codec.tag()), Ok(codec));
        }
    }

    #[test]
    fn bad_tag_rejected() {
        assert_eq!(Codec::from_tag(99), Err(EncodingError::BadHeader));
    }
}
