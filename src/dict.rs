use crate::count::{CountError, CountSet};
use rayon::iter::plumbing::{Consumer, UnindexedConsumer};
use rayon::prelude::*;
use std::collections::{HashMap, HashSet};

type OffsetLength = (usize, usize);

#[derive(Debug)]
pub struct Dictionary {
    word_string: String,
    word_count: HashMap<OffsetLength, CountSet>,
    word_set: HashSet<Box<str>>,
}

impl Dictionary {
    pub fn new() -> Self {
        Self {
            word_string: String::new(),
            word_count: HashMap::new(),
            word_set: HashSet::new(),
        }
    }

    // for use in file reading *only*
    pub unsafe fn from_raw_parts(
        word_string: String,
        word_count: HashMap<OffsetLength, CountSet>,
    ) -> Self {
        Self {
            word_string,
            word_count,
            word_set: HashSet::new(),
        }
    }

    pub fn add(&mut self, word: &str) -> Result<(), CountError> {
        if !self.word_set.contains(&Box::from(word)) {
            let offset = self.word_string.len();
            let len = word.len();

            self.word_string.push_str(word);
            self.word_count
                .insert((offset, len), CountSet::from_word(word)?);
            self.word_set.insert(Box::from(word));
        }

        Ok(())
    }

    #[inline]
    pub fn word_string(&self) -> &str {
        &self.word_string
    }

    #[inline]
    pub fn word_count(&self) -> &HashMap<OffsetLength, CountSet> {
        &self.word_count
    }

    #[inline]
    pub fn len(&self) -> usize {
        self.word_count.len()
    }

    #[inline]
    pub fn par_iter(&self) -> ParDictionaryIter {
        ParDictionaryIter { dict: self }
    }
}

pub struct DictionaryEntry<'a> {
    pub word: &'a str,
    pub count_set: &'a CountSet,
}

pub struct ParDictionaryIter<'a> {
    dict: &'a Dictionary,
}

impl<'a> ParallelIterator for ParDictionaryIter<'a> {
    type Item = DictionaryEntry<'a>;

    fn drive_unindexed<C>(self, consumer: C) -> <C as Consumer<Self::Item>>::Result
    where
        C: UnindexedConsumer<Self::Item>,
    {
        let par_iter = self
            .dict
            .word_count
            .par_iter()
            .map(|(&(offset, len), set)| DictionaryEntry {
                word: &self.dict.word_string[offset..(offset + len)],
                count_set: set,
            });

        par_iter.drive_unindexed(consumer)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sanity_check() {
        let mut dict = Dictionary::new();
        dict.add("ladies").unwrap();
        dict.add("and").unwrap();
        dict.add("gentlemen").unwrap();

        assert_eq!(dict.len(), 3);
    }

    #[test]
    fn no_duplicates() {
        let mut dict = Dictionary::new();
        dict.add("the").unwrap();
        dict.add("mitochondria").unwrap();
        dict.add("is").unwrap();
        dict.add("the").unwrap();
        dict.add("powerhouse").unwrap();
        dict.add("of").unwrap();
        dict.add("the").unwrap();
        dict.add("cell").unwrap();

        // the, mitochondria, is, powerhouse, of, cell
        assert_eq!(dict.len(), 6);
    }

    #[test]
    fn errors() {
        let mut dict = Dictionary::new();
        let err = dict.add("brøther").unwrap_err();

        match err {
            CountError::NotAscii => {}
            _ => panic!("Wrong 'not_ascii' error!"),
        }

        dict.add("may").unwrap();
        dict.add("i").unwrap();
        dict.add("have").unwrap();
        dict.add("some").unwrap();

        let err = dict.add("lööps").unwrap_err();

        match err {
            CountError::NotAscii => {}
            _ => panic!("Wrong 'not_ascii' error!"),
        }

        assert_eq!(dict.len(), 4);
    }
}
