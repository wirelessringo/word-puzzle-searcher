mod count;
mod dict;
mod format;

use crate::count::CountSet;
use crate::dict::Dictionary;
use crate::format::{read_dict, write_dict};
use rayon::prelude::*;
use std::error::Error;
use std::fs::{File, OpenOptions};
use std::io::{BufRead, BufReader};
use std::path::PathBuf;
use structopt::StructOpt;

#[derive(StructOpt, Debug)]
#[structopt(
    name = "Word Puzzle Searcher",
    about = "A search program for word puzzle games"
)]
enum Opt {
    /// Generates a dictionary file
    Generate {
        /// Output file
        #[structopt(short, long, parse(from_os_str), default_value = "default.dict")]
        output: PathBuf,

        /// File containing a list of words separated in lines
        #[structopt(name = "FILE", parse(from_os_str))]
        file: PathBuf,
    },
    /// Searches for words given a list of letters
    Search {
        /// Dictionary file
        #[structopt(short, long, parse(from_os_str), default_value = "default.dict")]
        dictionary: PathBuf,

        /// Available letters in the word puzzle
        letters: String,

        /// Minimum length of the words to be searched
        #[structopt(short, long, default_value = "3")]
        min_length: usize,

        /// Minimum length of the words to be searched
        #[structopt(short = "M", long)]
        max_length: Option<usize>,

        /// Separator for the list of words
        #[structopt(short, long, default_value = "\n")]
        separator: String,
    },
}

fn main() -> Result<(), Box<dyn Error>> {
    let opt = Opt::from_args();

    match opt {
        Opt::Generate { output, file } => {
            println!(
                "Generating a dictionary file ({:?}) from {:?}...",
                output, file
            );
            let file = File::open(&file)?;
            let file = BufReader::new(file);
            let mut dict = Dictionary::new();

            for line in file.lines() {
                let line = line?;
                dict.add(&line)?;
            }

            let mut output_file = OpenOptions::new().create(true).write(true).open(&output)?;

            write_dict(&dict, &mut output_file)?;
            println!("Generated dictionary file {:?}", output);
        }
        Opt::Search {
            dictionary,
            letters,
            min_length,
            max_length,
            separator,
        } => {
            println!("Using dictionary file {:?}...", dictionary);
            let mut dict_file = File::open(&dictionary)?;
            let dict = read_dict(&mut dict_file)?;

            println!(
                "Solving for string {:?}, with minimum length of {}{}",
                letters,
                min_length,
                if let Some(max_length) = max_length {
                    format!(", and maximum length of {}", max_length)
                } else {
                    String::new()
                }
            );

            let letter_count = CountSet::from_word(&letters)?;
            let mut words = dict
                .par_iter()
                .filter(|entry| letter_count.contains(entry.count_set))
                .map(|entry| entry.word)
                .filter(|word| {
                    word.len() >= min_length
                        && (if let Some(max_length) = max_length {
                            word.len() <= max_length
                        } else {
                            true
                        })
                })
                .collect::<Vec<_>>();

            words.par_sort_unstable();
            words
                .iter()
                .for_each(|word| print!("{}{}", word, separator));
        }
    }

    Ok(())
}
