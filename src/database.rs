use anyhow::Result;
use memmap2::Mmap;
use memmap2::MmapOptions;
use std::fs::File;

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

#[allow(dead_code)]
pub enum Page<'a> {
    Interior {
        page_number: PageNumber,
        cell_area_offset: u16,
        cell_count: u32,
        cell_offset_list: &'a [u8], // A offset is represent by two sequential bytes in this list.
        right_child: u32,
    },
    Leaf {
        page_number: PageNumber,
        cell_area_offset: u16,
        cell_count: u32,
        cell_offset_list: &'a [u8],
    },
}

impl<'a> Page<'a> {
    fn parse(mmap: &'a Mmap, page_number: PageNumber, page_size: u32) -> Self {
        if page_size == 0 || page_number == 0 {
            panic!("invalid page size or page number")
        }

        let offset = if page_number == 1 {
            100
        } else {
            ((page_number - 1) * page_size) as usize
        };

        let page = &mmap[offset..offset + page_size as usize];
        let page_type = page[0];
        let cell_count = u16::from_be_bytes([page[3], page[4]]) as u32;
        let cell_area_offset = u16::from_be_bytes([page[5], page[6]]);

        // Assuming that we have no index tables.
        match page_type {
            0x05 => Self::Interior {
                page_number,
                cell_area_offset,
                cell_count,
                cell_offset_list: &page[12..12 + cell_count as usize * 2],
                right_child: u32::from_be_bytes([page[8], page[9], page[10], page[11]]),
            },
            0x0d => Self::Leaf {
                page_number,
                cell_area_offset,
                cell_count,
                cell_offset_list: &page[8..8 + cell_count as usize * 2],
            },
            _ => panic!("corrupt database"),
        }
    }
}
