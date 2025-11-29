use std::fs::File;

use anyhow::Result;
use memmap2::Mmap;
use memmap2::MmapOptions;

use crate::varint::read_varint;

pub type PageNumber = u32;

const MIN_PAGE_SIZE: u32 = 1 << 9; // 512
const MAX_PAGE_SIZE: u32 = 1 << 16; // 65536, 64 KB

#[derive(Debug)]
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
        Page::parse(self, page_number)
    }
}

// No support for index tables.
const PT_INTERIOR_TABLE: u8 = 0x05;
const PT_LEAF_TABLE: u8 = 0x0d;
const HDR_INTERIOR: usize = 12;
const HDR_LEAF: usize = 8;

#[allow(dead_code)]
#[derive(Clone, Copy, Debug)]
pub struct PageCommon<'a> {
    db: &'a Database,
    data: &'a [u8],
    size: u32,
    number: PageNumber,
    cell_area_offset: u16,
    cell_count: u32,
    cell_offset_list: &'a [u8],
}

#[derive(Clone, Copy, Debug)]
pub enum Page<'a> {
    Interior { common: PageCommon<'a>, right_child: u32 },
    Leaf { common: PageCommon<'a> },
}

impl<'a> Page<'a> {
    pub fn entries(&self) -> EntryIter<'_> {
        EntryIter::new(self.common().db, self.common().number)
    }

    fn parse(db: &'a Database, page_number: PageNumber) -> Self {
        assert!(db.page_size != 0 && page_number != 0);

        let (offset, page_data) = if page_number == 1 {
            (100, &db.mmap[..db.page_size as usize])
        } else {
            let offset = ((page_number - 1) * db.page_size) as usize;
            (offset, &db.mmap[offset..offset + db.page_size as usize])
        };
        let page_type = db.mmap[offset];
        let cell_count = u16::from_be_bytes([db.mmap[offset + 3], db.mmap[offset + 4]]) as u32;
        assert!(cell_count < Page::max_cell_count(db.page_size));
        let cell_area_offset = u16::from_be_bytes([db.mmap[offset + 5], db.mmap[offset + 6]]);
        // Assuming that we have no index tables.
        let (header_len, right_child) = match page_type {
            PT_INTERIOR_TABLE => {
                let rc = u32::from_be_bytes([
                    db.mmap[offset + 8],
                    db.mmap[offset + 9],
                    db.mmap[offset + 10],
                    db.mmap[offset + 11],
                ]);
                (HDR_INTERIOR, Some(rc))
            }
            PT_LEAF_TABLE => (HDR_LEAF, None),
            v => panic!("corrupt database, page type has value: 0x{v:x}"),
        };
        let cell_offset_len = (cell_count as usize) * 2;
        let cell_offset_list = &db.mmap[offset + header_len..offset + header_len + cell_offset_len];
        let common = PageCommon {
            db,
            data: page_data,
            size: db.page_size,
            number: page_number,
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

    fn parse_cell(&self, offset: u16) -> Cell {
        assert!(offset >= self.common().cell_area_offset);
        assert!((offset as u32) < self.common().size);

        let mut cell_content = &self.common().data[offset as usize..];
        match self {
            Self::Interior { .. } => {
                let left_child =
                    PageNumber::from_be_bytes([cell_content[0], cell_content[1], cell_content[2], cell_content[3]]);
                let (key, _) = read_varint(&mut &cell_content[4..]);
                Cell::Interior { left_child, key }
            }
            Self::Leaf { .. } => {
                let (payload_size, _) = read_varint(&mut cell_content);
                let (key, _) = read_varint(&mut cell_content);
                let payload = &cell_content[..payload_size as usize];
                Cell::Leaf(Entry {
                    payload_size: payload_size as u64,
                    key,
                    payload: payload.to_vec(),
                })
            }
        }
    }

    fn common(&self) -> &PageCommon<'_> {
        match self {
            Self::Interior { common, .. } | Self::Leaf { common } => common,
        }
    }

    fn cell_offset_list(&self) -> &[u8] {
        self.common().cell_offset_list
    }

    fn max_cell_count(page_size: u32) -> u32 {
        (page_size - 8) / 6
    }
}

#[allow(dead_code)]
pub enum Cell {
    Interior { left_child: PageNumber, key: i64 },
    Leaf(Entry),
}

#[allow(dead_code)]
#[derive(Clone, Debug)]
pub struct Entry {
    pub payload_size: u64,
    pub key: i64,
    pub payload: Vec<u8>,
}

const ITER_MAX_DEPTH: usize = 20;

#[derive(Clone, Copy)]
pub struct EntryIter<'a> {
    db: &'a Database,
    curr_page: Page<'a>,
    curr_cell: usize,
    parents: [Option<(PageNumber, usize)>; ITER_MAX_DEPTH - 1],
    last_parent: usize,
}

impl<'a> EntryIter<'a> {
    fn new(db: &'a Database, root_page: PageNumber) -> Self {
        Self {
            db,
            curr_page: db.get_page(root_page),
            curr_cell: 0,
            parents: [None; ITER_MAX_DEPTH - 1],
            last_parent: 0,
        }
    }

    fn move_to_child(&mut self, child: PageNumber) {
        assert!(self.last_parent < ITER_MAX_DEPTH - 1);
        self.parents[self.last_parent] = Some((self.curr_page.common().number, self.curr_cell));
        self.last_parent += 1;
        self.curr_cell = 0;
        self.curr_page = self.db.get_page(child);
    }

    fn move_to_parent(&mut self) {
        assert!(self.last_parent > 0);
        self.last_parent -= 1;
        (self.curr_page, self.curr_cell) = self.parents[self.last_parent]
            .map(|(p, c)| (self.db.get_page(p), c))
            .unwrap();
        self.parents[self.last_parent] = None;
    }
}

impl<'a> Iterator for EntryIter<'a> {
    type Item = Entry;

    fn next(&mut self) -> Option<Self::Item> {
        if self.curr_cell >= self.curr_page.cell_offset_list().len() {
            self.curr_cell += 1;
            if let Page::Interior { right_child, .. } = &self.curr_page {
                self.move_to_child(*right_child);
            } else {
                while self.curr_cell > self.curr_page.cell_offset_list().len() {
                    if self.last_parent == 0 {
                        return None;
                    }
                    self.move_to_parent();
                }
                return self.next();
            }
        }
        let raw_list = self.curr_page.cell_offset_list();
        let offset_bytes = &raw_list[self.curr_cell..self.curr_cell + 2];
        let offset = u16::from_be_bytes([offset_bytes[0], offset_bytes[1]]);
        let cell = self.curr_page.parse_cell(offset);
        self.curr_cell += 2;
        match cell {
            Cell::Interior { left_child, .. } => {
                self.move_to_child(left_child);
                self.next()
            }
            Cell::Leaf(data) => Some(data),
        }
    }
}
