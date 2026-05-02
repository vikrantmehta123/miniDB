//! Encoding library for column data.
//!
//! Codecs transform typed value slices into byte streams (and back) before
//! lz4 compression. Each codec is self-describing: encoded output carries
//! enough header for `decode` to reverse it without external metadata.

pub mod delta;
