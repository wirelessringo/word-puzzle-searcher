//! Letter counting module

use std::convert::{TryFrom, TryInto};
use std::fmt;
use std::error::Error;

// function to turn 26-based index to 13-based index/offset pair
#[inline(always)]
fn to_index_offset(index: u8) -> (usize, u8) {
    // the offset *should* be optimized by LLVM to something like `(index & 1) << 2`
    (index as usize / 2, index % 2 * 4)
}

/// Error type returned by this module.
#[derive(Debug)]
pub enum CountError {
    /// String contains non-ASCII characters
    NotAscii,
    /// String contains characters other than letters (numbers, symbols, etc.)
    NotAlphabetic,
    /// Letter counter exceeded the count limit
    CountOverflow,
}

impl fmt::Display for CountError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        use CountError::*;

        write!(f, "{}", match *self {
            NotAscii => "String contains non-ASCII characters",
            NotAlphabetic => "String contains characters other than letters (numbers, symbols, etc.)",
            CountOverflow => "Letter counter exceeded the count limit",
        })
    }
}

impl Error for CountError {}

// We assume that words only contain at most 15 instances of a letter.
// The longest word that I can think of - "pneumonultramicroscopicsilicovolcanoconosis"
// only has a maximum of 8 instances of a letter (the letter "o").
#[derive(Clone, Hash, PartialEq, Eq)]
pub struct CountSet([u8; 13]);

pub struct CountSetIter<'a> {
    count: &'a CountSet,
    index: usize,
}

impl CountSet {
    pub fn from_word(word: &str) -> Result<Self, CountError> {
        if !word.is_ascii() {
            return Err(CountError::NotAscii);
        }

        // We can treat characters as bytes, since we are guaranteed to have only
        // characters in the ASCII range.
        if !word.bytes().all(|b| b.is_ascii_alphabetic()) {
            return Err(CountError::NotAlphabetic);
        }

        // Create a temporary, convenient array for counting letters, then try
        // to convert it later to the more compact form.
        let mut count = [0u8; 26];

        word.bytes()
            .map(|b| b.to_ascii_uppercase())
            .try_for_each(|b| {
                let i = b as usize - 65;
                count[i] = count[i].checked_add(1)?;
                Some(())
            })
            .ok_or(CountError::CountOverflow)?;

        count.try_into()
    }

    // doesn't perform bounds checks, `index` must be between 0 and 25
    #[inline]
    unsafe fn index_unchecked(&self, index: u8) -> u8 {
        let (index, offset) = to_index_offset(index);
        (self.0[index] & (0b1111 << offset)) >> offset
    }

    #[inline]
    pub fn iter(&self) -> CountSetIter {
        CountSetIter {
            count: self,
            index: 0,
        }
    }

    #[inline]
    pub fn slice(&self) -> &[u8] {
        &self.0
    }

    pub fn contains(&self, other: &Self) -> bool {
        self.iter().zip(other.iter()).all(|(s, o)| s >= o)
    }
}

impl fmt::Debug for CountSet {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.debug_map()
            .entries(
                self.iter()
                    .enumerate()
                    .filter(|(_, c)| *c > 0)
                    .map(
                        |(i, c)| (
                            // SAFETY: We are guaranteed to have a valid character value,
                            // since `i + 65` is within the range of ASCII characters.
                            unsafe { core::char::from_u32_unchecked(i as u32 + 65) },
                            c,
                        )
                    )
            )
            .finish()
    }
}

impl From<[u8; 13]> for CountSet {
    fn from(array: [u8; 13]) -> Self {
        Self(array)
    }
}

impl From<CountSet> for [u8; 13] {
    fn from(count: CountSet) -> Self {
        count.0
    }
}

impl TryFrom<[u8; 26]> for CountSet {
    type Error = CountError;

    fn try_from(array: [u8; 26]) -> Result<Self, Self::Error> {
        let mut count = [0; 13];
        for (i, &c) in array.iter().enumerate() {
            if c > 15 {
                return Err(CountError::CountOverflow);
            }

            let (index, offset) = to_index_offset(i as u8);
            count[index] |= c << offset;
        }

        Ok(Self(count))
    }
}

impl From<CountSet> for [u8; 26] {
    fn from(count: CountSet) -> Self {
        let mut array = [0; 26];
        for (i, c) in count.iter().enumerate() {
            array[i] = c;
        }

        array
    }
}

impl<'a> Iterator for CountSetIter<'a> {
    type Item = u8;

    fn next(&mut self) -> Option<Self::Item> {
        if self.index > 25 {
            return None;
        }

        // SAFETY: We did a bounds check; it's fine.
        let value = unsafe { self.count.index_unchecked(self.index as u8) };
        self.index += 1;

        Some(value)
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        let size = 26 - self.index;
        (size, Some(size))
    }
}

impl<'a> ExactSizeIterator for CountSetIter<'a> {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sanity_test() {
        let counted_word = CountSet::from_word("hello").unwrap();
        let count_set = CountSet::try_from([
        //  a  b  c  d  e  f  g  h  i  j  k  l  m
            0, 0, 0, 0, 1, 0, 0, 1, 0, 0, 0, 2, 0,
        //  n  o  p  q  r  s  t  u  v  w  x  y  z
            0, 1, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
        ]).unwrap();
        assert_eq!(counted_word, count_set);

        let counted_word = CountSet::from_word(
            "pneumonultramicroscopicsilicovolcanoconosis"
        ).unwrap();
        let count_set = CountSet::try_from([
        //  a  b  c  d  e  f  g  h  i  j  k  l  m
            2, 0, 6, 0, 1, 0, 0, 0, 5, 0, 0, 3, 2,
        //  n  o  p  q  r  s  t  u  v  w  x  y  z
            4, 8, 2, 0, 2, 4, 1, 2, 1, 0, 0, 0, 0,
        ]).unwrap();
        assert_eq!(counted_word, count_set);
    }

    #[test]
    fn not_ascii() {
        let error = CountSet::from_word("こんにちは").unwrap_err();
        match error {
            CountError::NotAscii => {},
            _ => panic!("Wrong 'not_ascii' error! {:?}", error),
        }
    }

    #[test]
    fn not_alphabetic() {
        let error = CountSet::from_word("hello world 123 !@#").unwrap_err();
        match error {
            CountError::NotAlphabetic => {},
            _ => panic!("Wrong 'not_alphabetic' error! {:?}", error),
        }
    }

    #[test]
    fn count_overflow() {
        let error = CountSet::from_word("aaaaaaaaaaaaaaaa").unwrap_err();
        match error {
            CountError::CountOverflow => {},
            _ => panic!("Wrong 'count_overflow' error! {:?}", error),
        }
    }
}
