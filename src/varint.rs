use anyhow::Result;
use std::io::SeekFrom;
use std::io::prelude::{Read, Seek};

pub fn read_varint_from_offset(reader: &mut (impl Seek + Read), offset: u64) -> Result<i64> {
    reader.seek(SeekFrom::Start(offset))?;
    read_varint(reader)
}

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
