use std::fs::File;

use anyhow::Result;
use memmap2::Mmap;
use memmap2::MmapOptions;

use crate::varint::read_varint;

pub type PageNumber = u32;

const MIN_PAGE_SIZE: u32 = 1 << 9; // 512
const MAX_PAGE_SIZE: u32 = 1 << 16; // 65536, 64 KB

pub struct Database {
    pub mmap: Mmap,
    pub page_size: u32,
    pub page_count: PageNumber,
}

impl Database {
    pub fn open(file: &File) -> Result<Self> {
        // SAFETY: In Wedson we trust 🙏
        let mmap = unsafe { MmapOptions::new().map(file)? };
        let page_size = u16::from_be_bytes([mmap[16], mmap[17]]) as u32;
        assert!((MIN_PAGE_SIZE..=MAX_PAGE_SIZE).contains(&page_size));
        assert!(page_size.is_power_of_two());
        let page_count = PageNumber::from_be_bytes([mmap[28], mmap[29], mmap[30], mmap[31]]);
        Ok(Self {
            mmap,
            page_size,
            page_count,
        })
    }

    pub fn get_page(&self, page_number: PageNumber) -> Page<'_> {
        assert!(page_number <= self.page_count);
        Page::parse(&self.mmap, page_number, self.page_size)
    }
}

// No support for index tables.
const PT_INTERIOR_TABLE: u8 = 0x05;
const PT_LEAF_TABLE: u8 = 0x0d;
const HDR_INTERIOR: usize = 12;
const HDR_LEAF: usize = 8;

#[allow(dead_code)]
pub struct PageCommon<'a> {
    pub page_data: &'a [u8],
    pub page_size: u32,
    pub page_number: PageNumber,
    pub cell_area_offset: u16,
    pub cell_count: u32,
    cell_offset_list: &'a [u8],
}

impl PageCommon<'_> {
    pub fn cell_offset_list(&self) -> impl Iterator<Item = u16> {
        self.cell_offset_list
            .chunks_exact(2)
            .map(|bs| u16::from_be_bytes([bs[0], bs[1]]))
    }
}

#[allow(dead_code)]
pub enum Page<'a> {
    Interior {
        common: PageCommon<'a>,
        right_child: u32,
    },
    Leaf {
        common: PageCommon<'a>,
    },
}

impl<'a> Page<'a> {
    pub fn parse(mmap: &'a Mmap, page_number: PageNumber, page_size: u32) -> Self {
        assert!(page_size != 0 && page_number != 0);

        let (offset, page_data) = if page_number == 1 {
            (100, &mmap[..page_size as usize])
        } else {
            let offset = ((page_number - 1) * page_size) as usize;
            (offset, &mmap[offset..offset + page_size as usize])
        };
        let page_type = mmap[offset];
        let cell_count = u16::from_be_bytes([mmap[offset + 3], mmap[offset + 4]]) as u32;
        assert!(cell_count < Page::max_cell_count(page_size));
        let cell_area_offset = u16::from_be_bytes([mmap[offset + 5], mmap[offset + 6]]);
        // Assuming that we have no index tables.
        let (header_len, right_child) = match page_type {
            PT_INTERIOR_TABLE => {
                let rc = u32::from_be_bytes([
                    mmap[offset + 8],
                    mmap[offset + 9],
                    mmap[offset + 10],
                    mmap[offset + 11],
                ]);
                (HDR_INTERIOR, Some(rc))
            }
            PT_LEAF_TABLE => (HDR_LEAF, None),
            v => panic!("corrupt database, page type has value: 0x{v:x}"),
        };
        let cell_offset_len = (cell_count as usize) * 2;
        let cell_offset_list = &mmap[offset + header_len..offset + header_len + cell_offset_len];
        let common = PageCommon {
            page_data,
            page_size,
            page_number,
            cell_area_offset,
            cell_count,
            cell_offset_list,
        };
        match right_child {
            Some(rc) => Self::Interior {
                common,
                right_child: rc,
            },
            None => Self::Leaf { common },
        }
    }

    pub fn parse_cell(&self, offset: u16) -> CellInfo<'_> {
        assert!(offset >= self.common().cell_area_offset);
        assert!((offset as u32) < self.common().page_size);

        let mut cell_content = &self.common().page_data[offset as usize..];
        match self {
            Self::Interior { .. } => {
                let left_child = PageNumber::from_be_bytes([
                    cell_content[0],
                    cell_content[1],
                    cell_content[2],
                    cell_content[3],
                ]);
                let (rowid, _) = read_varint(&mut &cell_content[4..]);
                CellInfo::Interior { left_child, rowid }
            }
            Self::Leaf { .. } => {
                let (payload_size, _) = read_varint(&mut cell_content);
                let (rowid, _) = read_varint(&mut cell_content);
                let payload = &cell_content[..payload_size as usize];
                CellInfo::Leaf {
                    payload_size: payload_size as u64,
                    rowid,
                    payload,
                }
            }
        }
    }

    pub fn common(&self) -> &PageCommon<'_> {
        match self {
            Self::Interior { common, .. } | Self::Leaf { common } => common,
        }
    }

    pub fn cell_offset_list(&self) -> impl Iterator<Item = u16> {
        self.common().cell_offset_list()
    }

    fn max_cell_count(page_size: u32) -> u32 {
        (page_size - 8) / 6
    }
}

#[allow(dead_code)]
pub enum CellInfo<'a> {
    Interior {
        left_child: PageNumber,
        rowid: i64,
    },
    Leaf {
        payload_size: u64,
        rowid: i64,
        payload: &'a [u8],
    },
}
