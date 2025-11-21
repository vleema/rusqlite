use anyhow::{Result, bail};

#[derive(Debug, Clone, Copy)]
pub enum TableType {
    Index,
    Table,
}

#[allow(dead_code)]
#[derive(Debug, Clone, Copy)]
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
