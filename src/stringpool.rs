use std::convert::TryFrom;
use std::io::SeekFrom;
use std::io::{Read, Seek};
use std::rc::Rc;

use crate::{read_u16, read_u32, read_u8, ChunkHeader, ParseError};

#[derive(Debug)]
pub(crate) struct StringPoolHeader {
    pub(crate) chunk_header: ChunkHeader,
    pub(crate) string_count: u32,
    pub(crate) style_count: u32,
    pub(crate) flags: u32,
    pub(crate) string_start: u32,
    pub(crate) style_start: u32,
}

impl StringPoolHeader {
    fn read_from_file<F: Read + Seek>(
        input: &mut F,
        chunk_header: &ChunkHeader,
    ) -> Result<Self, ParseError> {
        let chunk_header = chunk_header.clone();
        let string_count = read_u32(input)?;
        let style_count = read_u32(input)?;
        let flags = read_u32(input)?;

        let string_start = read_u32(input)?;
        let style_start = read_u32(input)?;

        let header = Self {
            chunk_header,
            string_count,
            style_count,
            flags,
            string_start,
            style_start,
        };

        // println!("{:?}", header);
        Ok(header)
    }
}

#[derive(Debug)]
pub(crate) struct StringPool {
    pub(crate) header: StringPoolHeader,
    pub(crate) strings: Vec<Rc<String>>,
}

impl StringPool {
    pub(crate) fn read_from_file<F: Read + Seek>(
        input: &mut F,
        chunk_header: &ChunkHeader,
    ) -> Result<Self, ParseError> {
        let string_pool_header = StringPoolHeader::read_from_file(input, chunk_header)?;
        assert_eq!(string_pool_header.style_count, 0);

        let flag_is_utf8 = (string_pool_header.flags & (1 << 8)) != 0;

        // Save current position in the file stream
        let chunk_data_start = input.stream_position().unwrap();

        // Parse string offsets
        let mut offsets =
            Vec::with_capacity(usize::try_from(string_pool_header.string_count).unwrap());
        for _ in 0..string_pool_header.string_count {
            offsets.push(read_u32(input)?);
        }

        const STRINGPOOL_HEADER_SIZE: usize = std::mem::size_of::<StringPoolHeader>();

        let s = string_pool_header.string_start - u32::try_from(STRINGPOOL_HEADER_SIZE).unwrap();
        input.seek(SeekFrom::Start(chunk_data_start)).unwrap();
        input.seek(SeekFrom::Current(s.into())).unwrap();

        // Save current position in the file stream
        let string_data_start = input.stream_position().unwrap();

        let mut strings =
            Vec::with_capacity(usize::try_from(string_pool_header.string_count).unwrap());
        for offset in offsets {
            input.seek(SeekFrom::Current(offset.into())).unwrap();

            if flag_is_utf8 {
                strings.push(Rc::new(parse_utf8_string(input)?));
            } else {
                strings.push(Rc::new(parse_utf16_string(input)?));
            }

            input.seek(SeekFrom::Start(string_data_start)).unwrap();
        }

        let s =
            string_pool_header.chunk_header.size - u32::try_from(STRINGPOOL_HEADER_SIZE).unwrap();
        input.seek(SeekFrom::Start(chunk_data_start)).unwrap();
        input.seek(SeekFrom::Current(s.into())).unwrap();

        Ok(Self {
            header: string_pool_header,
            strings,
        })
    }

    pub(crate) fn get(&self, i: usize) -> Option<Rc<String>> {
        if u32::try_from(i).unwrap() == u32::MAX {
            return None;
        }

        Some(self.strings.get(i).unwrap().clone())
    }
}

fn parse_utf16_string<F: Read + Seek>(input: &mut F) -> Result<String, ParseError> {
    let len = read_u16(input)?;

    // Handles the case where the string is > 32767 characters
    if is_high_bit_set_16(len) {
        unimplemented!()
    }

    let mut s = Vec::with_capacity(len.into());
    for _ in 0..len {
        s.push(read_u16(input)?);
    }

    // Encoded string length does not include the trailing 0
    let _ = read_u16(input)?;

    Ok(String::from_utf16(&s).unwrap())
}

fn is_high_bit_set_16(input: u16) -> bool {
    input & (1 << 15) != 0
}

fn parse_utf8_string<F: Read + Seek>(input: &mut F) -> Result<String, ParseError> {
    let _ = read_u8(input)?;
    let len = read_u8(input)?;

    // Handles the case where the length value has high bit set
    // Not quite clear if the UTF-8 encoding actually has this but
    // perform the check anyway...
    if is_high_bit_set_8(len) {
        unimplemented!()
    }

    let mut s = Vec::with_capacity(len.into());
    for _ in 0..len {
        s.push(read_u8(input)?);
    }

    // Encoded string length does not include the trailing 0
    let _ = read_u8(input)?;

    Ok(String::from_utf8(s).unwrap())
}

fn is_high_bit_set_8(input: u8) -> bool {
    input & (1 << 7) != 0
}
