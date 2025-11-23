//! Implements datatypes and functionalities for B-tree pages.
//!
//! A b-tree page is either an interior page or a leaf page. A leaf page contains keys and in the case of a
//! table b-tree each key has associated data. An interior page contains K keys together with K+1 pointers to
//! child b-tree pages. A "pointer" in an interior b-tree page is just the 32-bit unsigned integer page number
//! of the child page.
//!
//! Each entry in a table b-tree consists of a 64-bit signed integer key and up to 2147483647 bytes of
//! arbitrary data. (The key of a table b-tree corresponds to the rowid of the SQL table that the b-tree
//! implements.) Interior table b-trees hold only keys and pointers to children. All data is contained in the
//! table b-tree leaves.
//!
//! When the size of payload for a cell exceeds a certain threshold (to be defined later) then only the first
//! few bytes of the payload are stored on the b-tree page and the balance is stored in a linked list of
//! content overflow pages.
//!
//! A b-tree page is divided into regions in the following order:
//!
//! 1. The 100-byte database file header (found on page 1 only)
//! 2. The 8 or 12 byte b-tree page header
//! 3. The cell pointer array
//! 4. Unallocated space
//! 5. The cell content area
//! 6. The reserved region
//!
//! The 100-byte database file header is found only on page 1, which is always a table b-tree page. All other
//! b-tree pages in the database file omit this 100-byte header.
//!
//! The b-tree page header is 8 bytes in size for leaf pages and 12 bytes for interior pages. All multibyte
//! values in the page header are big-endian. The b-tree page header is composed of the following fields:
//!
//! | Offset | Size | Description                                                                                                                          |
//! |--------|------|--------------------------------------------------------------------------------------------------------------------------------------|
//! | 0      | 1    | The one-byte flag at offset 0 indicating the b-tree page type:                                                                       |
//! |        |      |   • 2 (0x02): interior index b-tree page                                                                                             |
//! |        |      |   • 5 (0x05): interior table b-tree page                                                                                             |
//! |        |      |   • 10 (0x0a): leaf index b-tree page                                                                                                |
//! |        |      |   • 13 (0x0d): leaf table b-tree page                                                                                                |
//! |        |      | Any other value is an error.                                                                                                         |
//! | 1      | 2    | The two-byte integer at offset 1 gives the start of the first freeblock on the page, or zero if none.                                |
//! | 3      | 2    | The two-byte integer at offset 3 gives the number of cells on the page.                                                              |
//! | 5      | 2    | The two-byte integer at offset 5 designates the start of the cell content area. Zero is interpreted as 65536.                        |
//! | 7      | 1    | The one-byte integer at offset 7 gives the number of fragmented free bytes within the cell content area.                             |
//! | 8      | 4    | The four-byte page number at offset 8 is the right-most pointer. Present only in interior b-tree pages; omitted for all other pages. |
//!
//! The cell pointer array of a b-tree page immediately follows the b-tree page header. Let K be the number of
//! cells on the btree. The cell pointer array consists of K 2-byte integer offsets to the cell contents. The
//! cell pointers are arranged in key order with left-most cell (the cell with the smallest key) first and the
//! right-most cell (the cell with the largest key) last. Cell content is stored in the cell content region
//! of the b-tree page.
//!
//! Cells in a page B-tree follows the format:
//!
//! | Datatype   | Table Leaf (0x0d) | Index Leaf (0x05) | Index Leaf (0x0a) | Index Interior (0x02)| Description     |
//! |:-----------|:-----------------:|:-----------------:|:-----------------:|:--------------------:|:----------------|
//! | 4-byte int |                   |                   | x                 | x                    | Left child page |
//! | varint     | x                 |                   | x                 | x                    | Payload size    |
//! | varint     | x                 | x                 |                   |                      | Rowid           |
//! | bytes      | x                 |                   | x                 | x                    | Payload         |
//! | 4-byte int | x                 |                   | x                 | x                    | Overflow page   |
//!
//! Define the "payload" of a cell to be the arbitrary length section of the cell. For an index b-tree, the
//! key is always arbitrary in length and hence the payload is the key. There are no arbitrary length elements
//! in the cells of interior table b-tree pages and so those cells have no payload. Table b-tree leaf pages
//! contain arbitrary length content and so for cells on those pages the payload is the content.
//!
//! The payload is structured in something called the Record Format.
//!
//! A record contains a header and a body, in that order. The header begins with a single varint which
//! determines the total number of bytes in the header. The varint value is the size of the header in bytes
//! including the size varint itself. Following the size varint are one or more additional varints, one per
//! column. These additional varints are called "serial type" numbers and determine the datatype of each
//! column, according to the following chart:
//!
//! | Serial Type    | Content Size | Meaning                                                                                                |
//! |----------------|--------------|--------------------------------------------------------------------------------------------------------|
//! | 0              | 0            | Value is a NULL.                                                                                       |
//! | 1              | 1            | Value is an 8-bit twos-complement integer.                                                             |
//! | 2              | 2            | Value is a big-endian 16-bit twos-complement integer.                                                  |
//! | 3              | 3            | Value is a big-endian 24-bit twos-complement integer.                                                  |
//! | 4              | 4            | Value is a big-endian 32-bit twos-complement integer.                                                  |
//! | 5              | 6            | Value is a big-endian 48-bit twos-complement integer.                                                  |
//! | 6              | 8            | Value is a big-endian 64-bit twos-complement integer.                                                  |
//! | 7              | 8            | Value is a big-endian IEEE 754-2008 64-bit floating point number.                                      |
//! | 8              | 0            | Value is the integer 0. (Only available for schema format 4 and higher.)                               |
//! | 9              | 0            | Value is the integer 1. (Only available for schema format 4 and higher.)                               |
//! | 10,11          | variable     | Reserved for internal use. These serial type codes will never appear in a well-formed database file.   |
//! | ≥ 12 and even  | (N-12)/2     | Value is a BLOB that is (N-12)/2 bytes in length.                                                      |
//! | ≥ 13 and odd   | (N-13)/2     | Value is a string in the text encoding and (N-13)/2 bytes in length. The nul terminator is not stored. |
//!
//! Taken from <https://sqlite.org/fileformat2.html>, more information there.
use crate::read_varint;
use anyhow::{Result, bail};

/// Represents a b-tree page table type, either a index table or a regular table.
#[derive(Debug, Clone, Copy)]
pub enum TableType {
    Index,
    Table,
}

#[allow(dead_code)]
#[derive(Debug, Clone, Copy)]
/// Reprents a SQLite page type, either a interior b-tree page or leaf b-tree page.
///
/// A page type is encoded as:
///  - 0x02 for interior index b-tree page
///  - 0x05 for interior table b-tree page
///  - 0x0a for leaf index b-tree page
///  - 0x0d for leaf table b-tree page
///
/// Implements `From<u8>`, will panic if passed `u8` is invalid.
pub enum PageType {
    Interior(TableType),
    Leaf(TableType),
}

impl From<u8> for PageType {
    fn from(value: u8) -> Self {
        use PageType::*;
        use TableType::*;
        match value {
            0x02 => Interior(Index),
            0x05 => Interior(Table),
            0x0a => Leaf(Index),
            0x0d => Leaf(Table),
            // HACK: Instead of panicking, maybe return a Result somehow.
            _ => panic!("invalid u8 value for PageType"),
        }
    }
}

/// Represents a SQLite page header.
#[allow(dead_code)]
#[derive(Debug, Clone, Copy)]
pub struct PageHeader {
    pub ty: PageType,
    pub fst_freeblock: u16,
    pub number_of_cells: u16,
    pub cell_content_area_start: u16,
    pub fragmented_bytes: u8,
    pub rightmost_ptr: Option<u32>,
}

impl PageHeader {
    /// Creates a new header
    pub fn new(bytes: &[u8]) -> Result<Self> {
        if bytes.is_empty() {
            bail!("invalid page header")
        }
        let page_type = PageType::from(bytes[0]);
        let rightmost_ptr = match page_type {
            PageType::Interior(..) => Some(u32::from_be_bytes([
                bytes[8], bytes[9], bytes[10], bytes[11],
            ])),
            _ => None,
        };
        Ok(Self {
            ty: page_type,
            fst_freeblock: u16::from_be_bytes([bytes[1], bytes[2]]),
            number_of_cells: u16::from_be_bytes([bytes[3], bytes[4]]),
            cell_content_area_start: u16::from_be_bytes([bytes[5], bytes[6]]),
            fragmented_bytes: bytes[7],
            rightmost_ptr,
        })
    }
}

/// Represents the serial type encoding.
///
/// Implements `From<u64>`.
#[allow(dead_code)]
pub enum SerialType {
    Null,
    Int8,
    Int16,
    Int24,
    Int32,
    Int48,
    Int64,
    Float,
    Zero,
    One,
    Internal,
    Blob { size: u64 },
    Text { size: u64 },
}

impl From<u64> for SerialType {
    fn from(value: u64) -> Self {
        use SerialType::*;
        match value {
            0 => Null,
            1 => Int8,
            2 => Int16,
            3 => Int24,
            4 => Int32,
            5 => Int48,
            6 => Int64,
            7 => Float,
            8 => Zero,
            9 => One,
            10 | 11 => Internal,
            n if n % 2 == 0 => Blob { size: (n - 12) / 2 },
            n => Text { size: (n - 13) / 2 },
        }
    }
}

/// Represents the [sqlite_schema](https://sqlite.org/schematab.html) cell structure.
#[allow(dead_code)]
#[derive(Debug, Clone, Copy)]
pub struct SchemaCell<'a> {
    pub ty: &'a str,
    pub name: &'a str,
    pub tbl_name: &'a str,
    pub rootpage: u64,
    pub sql: &'a str,
}

impl<'a> SchemaCell<'a> {
    pub fn new(mut payload: &'a [u8]) -> Result<Self> {
        use SerialType::*;

        let header_size = read_varint(&mut payload)? as usize;

        let mut cursor = &payload[..header_size];

        let Text { size: ty_size } = SerialType::from(read_varint(&mut cursor)? as u64) else {
            bail!("invalid serial type for type")
        };
        let Text { size: name_size } = SerialType::from(read_varint(&mut cursor)? as u64) else {
            bail!("invalid serial type for name")
        };
        let Text { size: tbl_size } = SerialType::from(read_varint(&mut cursor)? as u64) else {
            bail!("invalid serial type for tbl_name")
        };
        _ = SerialType::from(read_varint(&mut cursor)? as u64);
        let Text { size: _sql_size } = SerialType::from(read_varint(&mut cursor)? as u64) else {
            bail!("invalid serial type for sql")
        };

        cursor = &payload[header_size as usize - 1..];
        let (ty, cursor) = next_utf8(cursor, ty_size as usize)?;
        let (name, cursor) = next_utf8(cursor, name_size as usize)?;
        let (tbl_name, _cursor) = next_utf8(cursor, tbl_size as usize)?;
        // TODO:
        // let rootpage = u64::from_be_bytes([cursor[])
        // let sql = ...

        Ok(Self {
            ty,
            name,
            tbl_name,
            rootpage: 42, // stub
            sql: name,    // stub
        })
    }
}

fn next_utf8(v: &[u8], size: usize) -> Result<(&str, &[u8])> {
    Ok((std::str::from_utf8(&v[..size])?, &v[size..]))
}
