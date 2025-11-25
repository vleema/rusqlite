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
use std::io::Read;

/// Reads a varint from the current position of the given reader.
///
/// Returns the parsed number and the varint length.
pub fn read_varint(buf: &mut impl Read) -> (i64, u8) {
    let mut ret = 0i64;
    let mut b = [0; 1];

    let mut i = 1;
    while i <= 9 {
        let Ok(()) = buf.read_exact(&mut b) else {
            break;
        };

        ret = ret << 7 | (b[0] & !(1 << 7)) as i64;
        if b[0] >> 7 == 0 {
            break;
        }
        i += 1;
    }

    (ret, i)
}
