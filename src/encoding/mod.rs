//! Encoding library for column data.
//!
//! Codecs transform typed value slices into byte streams (and back) before
//! lz4 compression. Each codec is self-describing: encoded output carries
//! enough header for `decode` to reverse it without external metadata.

pub mod delta;

mod sealed {
    pub trait Sealed {}
}

pub trait Primitive: Copy + sealed::Sealed {
    const WIDTH: usize;

    fn encode_le(self, out: &mut Vec<u8>);
    fn decode_le(bytes: &[u8]) -> Self;
    fn wrapping_sub(self, rhs: Self) -> Self;
    fn wrapping_add(self, rhs: Self) -> Self;
}

macro_rules! impl_primitive {
    ($($ty:ty),+ $(,)?) => {
        $(
            impl sealed::Sealed for $ty {}

            impl Primitive for $ty {
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
        )+
    };
}

impl_primitive!(i8, i16, i32, i64, u8, u16, u32, u64);
