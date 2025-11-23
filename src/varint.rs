//! Implements varint parsing and reading functionalities.
//!
//! A variable-length integer or "varint" is a static Huffman encoding of 64-bit twos-complement integers
//! that uses less space for small positive values. A varint is between 1 and 9 bytes in length.
//! The varint consists of either zero or more bytes which have the high-order bit set followed by a
//! single byte with the high-order bit clear, or nine bytes, whichever is shorter. The lower seven bits
//! of each of the first eight bytes and all 8 bits of the ninth byte are used to reconstruct the 64-bit twos-
//! complement integer. Varints are big-endian: bits taken from the earlier byte of the varint are more
//! significant than bits taken from the later bytes.
//!
//! The following table tries to make clear the storage principle of varint values:
//!
//! | Bytes | Varint                       | Maximum integer   |
//! |-------|------------------------------|-------------------|
//! | 1     | 0XXXXXXX                     | 127               |
//! | 2     | 1XXXXXXX 0XXXXXXX            | 16384             |
//! | 3     | 1XXXXXXX 1XXXXXXX 0XXXXXXX   | 2097152           |
//! | ...   | ...                          | ...               |
//!
//!
//! Taken from <https://sqlite.org/fileformat2.html>, more information there.
use anyhow::Result;
use std::io::SeekFrom;
use std::io::prelude::{Read, Seek};

/// Reads a varint from the given reader, first seeking to the specified offset.
///
/// Returns the 64-bit signed integer value of the varint on sucess, or a anyhow::Error if an I/O
/// occurs during seeking, or if the varint is malformed.
pub fn read_varint_from_offset(reader: &mut (impl Seek + Read), offset: u64) -> Result<i64> {
    reader.seek(SeekFrom::Start(offset))?;
    read_varint(reader)
}

/// Reads a varint from the current position of the given reader.
///
/// Returns the 64-bit signed integer value of the varint on sucess, or a `anyhow::Error` if the varint is malformed.
pub fn read_varint(reader: &mut impl Read) -> Result<i64> {
    let mut ret = 0i64;
    let mut b = [0; 1];

    for _ in 0..9 {
        reader.read_exact(&mut b)?;

        ret = ret << 7 | (b[0] & !(1 << 7)) as i64;

        if b[0] >> 7 == 0 {
            break;
        }
    }

    Ok(ret)
}
