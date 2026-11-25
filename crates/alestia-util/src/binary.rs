//! Bounds-checked little-endian and big-endian binary cursor readers and writers.
//!
//! Replaces unchecked C++ pointer casts (`src/celutil/binaryread.h`, `binarywrite.h`)
//! with explicit [`BinaryError::UnexpectedEof`] diagnostics.

use std::error::Error;
use std::fmt;

/// Errors returned by [`BinaryReader`] operations.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum BinaryError {
    /// Input buffer ended before the requested byte count could be read.
    UnexpectedEof {
        /// Number of bytes requested by the read operation.
        needed: usize,
        /// Number of unread bytes actually remaining.
        remaining: usize,
    },
}

impl fmt::Display for BinaryError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::UnexpectedEof { needed, remaining } => {
                write!(
                    f,
                    "unexpected EOF: needed {needed} bytes, only {remaining} remaining"
                )
            }
        }
    }
}

impl Error for BinaryError {}

/// Safe, zero-copy cursor for reading little-endian and big-endian primitives from `&[u8]`.
#[derive(Clone, Debug)]
pub struct BinaryReader<'a> {
    data: &'a [u8],
    pos: usize,
}

impl<'a> BinaryReader<'a> {
    /// Create a [`BinaryReader`] positioned at byte offset `0`.
    #[must_use]
    pub const fn new(data: &'a [u8]) -> Self {
        Self { data, pos: 0 }
    }

    /// Current byte offset within the buffer.
    #[must_use]
    pub const fn position(&self) -> usize {
        self.pos
    }

    /// Number of unread bytes remaining.
    #[must_use]
    pub const fn remaining(&self) -> usize {
        self.data.len().saturating_sub(self.pos)
    }

    /// Read an exact slice of `len` bytes without copying.
    ///
    /// # Errors
    /// Returns [`BinaryError::UnexpectedEof`] if fewer than `len` bytes remain.
    pub fn read_bytes(&mut self, len: usize) -> Result<&'a [u8], BinaryError> {
        let remaining = self.remaining();
        if remaining < len {
            return Err(BinaryError::UnexpectedEof {
                needed: len,
                remaining,
            });
        }
        let slice = &self.data[self.pos..self.pos + len];
        self.pos += len;
        Ok(slice)
    }

    /// Read an exact fixed-size byte array `[u8; N]`.
    ///
    /// # Errors
    /// Returns [`BinaryError::UnexpectedEof`] if fewer than `N` bytes remain.
    pub fn read_array<const N: usize>(&mut self) -> Result<[u8; N], BinaryError> {
        let slice = self.read_bytes(N)?;
        let mut out = [0_u8; N];
        out.copy_from_slice(slice);
        Ok(out)
    }

    /// Read a single `u8`.
    ///
    /// # Errors
    /// Returns [`BinaryError::UnexpectedEof`] on EOF.
    pub fn read_u8(&mut self) -> Result<u8, BinaryError> {
        Ok(self.read_array::<1>()?[0])
    }

    /// Read a little-endian `u16`.
    ///
    /// # Errors
    /// Returns [`BinaryError::UnexpectedEof`] if fewer than 2 bytes remain.
    pub fn read_le_u16(&mut self) -> Result<u16, BinaryError> {
        Ok(u16::from_le_bytes(self.read_array::<2>()?))
    }

    /// Read a little-endian `i16`.
    ///
    /// # Errors
    /// Returns [`BinaryError::UnexpectedEof`] if fewer than 2 bytes remain.
    pub fn read_le_i16(&mut self) -> Result<i16, BinaryError> {
        Ok(i16::from_le_bytes(self.read_array::<2>()?))
    }

    /// Read a little-endian `u32`.
    ///
    /// # Errors
    /// Returns [`BinaryError::UnexpectedEof`] if fewer than 4 bytes remain.
    pub fn read_le_u32(&mut self) -> Result<u32, BinaryError> {
        Ok(u32::from_le_bytes(self.read_array::<4>()?))
    }

    /// Read a little-endian `i32`.
    ///
    /// # Errors
    /// Returns [`BinaryError::UnexpectedEof`] if fewer than 4 bytes remain.
    pub fn read_le_i32(&mut self) -> Result<i32, BinaryError> {
        Ok(i32::from_le_bytes(self.read_array::<4>()?))
    }

    /// Read a little-endian `u64`.
    ///
    /// # Errors
    /// Returns [`BinaryError::UnexpectedEof`] if fewer than 8 bytes remain.
    pub fn read_le_u64(&mut self) -> Result<u64, BinaryError> {
        Ok(u64::from_le_bytes(self.read_array::<8>()?))
    }

    /// Read a little-endian `i64`.
    ///
    /// # Errors
    /// Returns [`BinaryError::UnexpectedEof`] if fewer than 8 bytes remain.
    pub fn read_le_i64(&mut self) -> Result<i64, BinaryError> {
        Ok(i64::from_le_bytes(self.read_array::<8>()?))
    }

    /// Read a little-endian `f32`.
    ///
    /// # Errors
    /// Returns [`BinaryError::UnexpectedEof`] if fewer than 4 bytes remain.
    pub fn read_le_f32(&mut self) -> Result<f32, BinaryError> {
        Ok(f32::from_le_bytes(self.read_array::<4>()?))
    }

    /// Read a little-endian `f64`.
    ///
    /// # Errors
    /// Returns [`BinaryError::UnexpectedEof`] if fewer than 8 bytes remain.
    pub fn read_le_f64(&mut self) -> Result<f64, BinaryError> {
        Ok(f64::from_le_bytes(self.read_array::<8>()?))
    }

    /// Read a big-endian `u16`.
    ///
    /// # Errors
    /// Returns [`BinaryError::UnexpectedEof`] if fewer than 2 bytes remain.
    pub fn read_be_u16(&mut self) -> Result<u16, BinaryError> {
        Ok(u16::from_be_bytes(self.read_array::<2>()?))
    }

    /// Read a big-endian `u32`.
    ///
    /// # Errors
    /// Returns [`BinaryError::UnexpectedEof`] if fewer than 4 bytes remain.
    pub fn read_be_u32(&mut self) -> Result<u32, BinaryError> {
        Ok(u32::from_be_bytes(self.read_array::<4>()?))
    }

    /// Read a big-endian `i32`.
    ///
    /// # Errors
    /// Returns [`BinaryError::UnexpectedEof`] if fewer than 4 bytes remain.
    pub fn read_be_i32(&mut self) -> Result<i32, BinaryError> {
        Ok(i32::from_be_bytes(self.read_array::<4>()?))
    }

    /// Read a big-endian `u64`.
    ///
    /// # Errors
    /// Returns [`BinaryError::UnexpectedEof`] if fewer than 8 bytes remain.
    pub fn read_be_u64(&mut self) -> Result<u64, BinaryError> {
        Ok(u64::from_be_bytes(self.read_array::<8>()?))
    }

    /// Read a big-endian `f32`.
    ///
    /// # Errors
    /// Returns [`BinaryError::UnexpectedEof`] if fewer than 4 bytes remain.
    pub fn read_be_f32(&mut self) -> Result<f32, BinaryError> {
        Ok(f32::from_be_bytes(self.read_array::<4>()?))
    }

    /// Read a big-endian `f64`.
    ///
    /// # Errors
    /// Returns [`BinaryError::UnexpectedEof`] if fewer than 8 bytes remain.
    pub fn read_be_f64(&mut self) -> Result<f64, BinaryError> {
        Ok(f64::from_be_bytes(self.read_array::<8>()?))
    }
}

/// Owned byte buffer builder for writing little-endian and big-endian primitives.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct BinaryWriter {
    buf: Vec<u8>,
}

impl BinaryWriter {
    /// Create an empty [`BinaryWriter`].
    #[must_use]
    pub const fn new() -> Self {
        Self { buf: Vec::new() }
    }

    /// Borrow accumulated bytes.
    #[must_use]
    pub fn as_slice(&self) -> &[u8] {
        &self.buf
    }

    /// Consume the writer and return the owned byte vector.
    #[must_use]
    pub fn into_inner(self) -> Vec<u8> {
        self.buf
    }

    /// Append raw bytes.
    pub fn write_bytes(&mut self, bytes: &[u8]) {
        self.buf.extend_from_slice(bytes);
    }

    /// Append a `u8`.
    pub fn write_u8(&mut self, v: u8) {
        self.buf.push(v);
    }

    /// Append a little-endian `u16`.
    pub fn write_le_u16(&mut self, v: u16) {
        self.buf.extend_from_slice(&v.to_le_bytes());
    }

    /// Append a little-endian `u32`.
    pub fn write_le_u32(&mut self, v: u32) {
        self.buf.extend_from_slice(&v.to_le_bytes());
    }

    /// Append a little-endian `u64`.
    pub fn write_le_u64(&mut self, v: u64) {
        self.buf.extend_from_slice(&v.to_le_bytes());
    }

    /// Append a little-endian `f32`.
    pub fn write_le_f32(&mut self, v: f32) {
        self.buf.extend_from_slice(&v.to_le_bytes());
    }

    /// Append a little-endian `f64`.
    pub fn write_le_f64(&mut self, v: f64) {
        self.buf.extend_from_slice(&v.to_le_bytes());
    }

    /// Append a big-endian `u16`.
    pub fn write_be_u16(&mut self, v: u16) {
        self.buf.extend_from_slice(&v.to_be_bytes());
    }

    /// Append a big-endian `u32`.
    pub fn write_be_u32(&mut self, v: u32) {
        self.buf.extend_from_slice(&v.to_be_bytes());
    }

    /// Append a big-endian `u64`.
    pub fn write_be_u64(&mut self, v: u64) {
        self.buf.extend_from_slice(&v.to_be_bytes());
    }

    /// Append a big-endian `f32`.
    pub fn write_be_f32(&mut self, v: f32) {
        self.buf.extend_from_slice(&v.to_be_bytes());
    }

    /// Append a big-endian `f64`.
    pub fn write_be_f64(&mut self, v: f64) {
        self.buf.extend_from_slice(&v.to_be_bytes());
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn roundtrip_le_and_be_primitives() {
        let mut w = BinaryWriter::new();
        w.write_u8(42);
        w.write_le_u16(0x1234);
        w.write_le_u32(0xDEAD_BEEF);
        w.write_le_f32(1.25);
        w.write_be_u32(0xCAFE_BABE);
        w.write_be_f64(-3.5);

        let bytes = w.into_inner();
        let mut r = BinaryReader::new(&bytes);
        assert_eq!(r.read_u8(), Ok(42));
        assert_eq!(r.read_le_u16(), Ok(0x1234));
        assert_eq!(r.read_le_u32(), Ok(0xDEAD_BEEF));
        assert_eq!(r.read_le_f32(), Ok(1.25));
        assert_eq!(r.read_be_u32(), Ok(0xCAFE_BABE));
        assert_eq!(r.read_be_f64(), Ok(-3.5));
        assert_eq!(r.remaining(), 0);
        assert_eq!(
            r.read_u8(),
            Err(BinaryError::UnexpectedEof {
                needed: 1,
                remaining: 0
            })
        );
    }
}
