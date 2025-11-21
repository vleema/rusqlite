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
