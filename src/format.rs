//! Dictionary file format reading/parsing and writing

use std::collections::HashMap;
use std::fmt;
use std::io::{self, Write, Read, ErrorKind};
use std::error::Error;
use std::convert::TryInto;
use rayon::prelude::*;
use crate::count::CountSet;
use crate::dict::Dictionary;

const FORMAT_VERSION: u32 = 1;

const USIZE: usize = std::mem::size_of::<usize>();
const COUNT_SET_SIZE: usize = std::mem::size_of::<CountSet>();
const WORD_COUNT_STRIDE: usize = USIZE * 2 + COUNT_SET_SIZE;

/// Error type returned by the `read_dict` function
#[derive(Debug)]
pub enum ReadError {
    /// wrong format
    FormatError,
    /// Error returned by an I/O operation
    IoError(io::Error),
}

impl fmt::Display for ReadError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        use ReadError::*;

        if let IoError(err) = self {
            return fmt::Display::fmt(err, f);
        }

        write!(f, "{}", match *self {
            FormatError => "wrong format",
            _ => "",
        })
    }
}

impl Error for ReadError {}

impl From<io::Error> for ReadError {
    fn from(err: io::Error) -> Self {
        ReadError::IoError(err)
    }
}

pub fn read_dict<R: Read>(reader: &mut R) -> Result<Dictionary, ReadError> {
    let mut magic = [0; 4];
    reader.read_exact(&mut magic)?;
    if &magic != b"DICT" {
        return Err(ReadError::FormatError);
    }

    let mut version = [0; 4];
    reader.read_exact(&mut version)?;
    let version = u32::from_le_bytes(version);
    if version != FORMAT_VERSION {
        return Err(ReadError::FormatError);
    }

    let mut word_count_length = [0; USIZE];
    reader.read_exact(&mut word_count_length)?;
    let word_count_length = usize::from_le_bytes(word_count_length);

    let mut str_length = [0; USIZE];
    reader.read_exact(&mut str_length)?;
    let str_length = usize::from_le_bytes(str_length);

    let mut word_string = vec![0; str_length];
    reader.read_exact(&mut word_string)?;
    let word_string = String::from_utf8(word_string).map_err(|_| ReadError::FormatError)?;

    let mut word_count_buf = vec![0; word_count_length * WORD_COUNT_STRIDE];
    reader.read_exact(&mut word_count_buf)
        .map_err(|e| if e.kind() == ErrorKind::UnexpectedEof {
            ReadError::FormatError
        } else {
            ReadError::IoError(e)
        })?;

    let word_count = (0..word_count_length).into_par_iter()
        .map(|i| &word_count_buf[(i * WORD_COUNT_STRIDE)..((i + 1) * WORD_COUNT_STRIDE)])
        .map(|count_element| {
            let offset: [u8; USIZE] = (&count_element[0..USIZE]).try_into().unwrap();
            let offset = usize::from_le_bytes(offset);

            let len: [u8; USIZE] = (&count_element[USIZE..(USIZE * 2)]).try_into().unwrap();
            let len = usize::from_le_bytes(len);

            let set: [u8; COUNT_SET_SIZE] = (&count_element[(USIZE * 2)..(WORD_COUNT_STRIDE)]).try_into().unwrap();
            let set = CountSet::from(set);

            ((offset, len), set)
        })
        .collect::<HashMap<_, _>>();

    Ok(unsafe { Dictionary::from_raw_parts(word_string, word_count) })
}

pub fn write_dict<W: Write>(dict: &Dictionary, writer: &mut W) -> io::Result<()> {
    writer.write_all(b"DICT")?;
    writer.write_all(&FORMAT_VERSION.to_le_bytes())?;
    writer.write_all(&dict.len().to_le_bytes())?;
    writer.write_all(&dict.word_string().len().to_le_bytes())?;
    writer.write_all(dict.word_string().as_bytes())?;
    for (&(offset, len), set) in dict.word_count().iter() {
        writer.write_all(&offset.to_le_bytes())?;
        writer.write_all(&len.to_le_bytes())?;
        writer.write_all(set.slice())?;
    }

    Ok(())
}
