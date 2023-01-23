#![feature(iter_array_chunks)]
#![feature(array_methods)]
use std::{
    error::Error,
    fs::File,
    io::{self, BufRead},
    path::PathBuf,
};

use clap::{Parser, ValueEnum};

#[derive(Clone, ValueEnum)]
enum Mode {
    /// The first variant of the problem, where a single rucksack is split into two to search for the common item
    Single,
    /// The second variant of the problem, where three rucksacks are searched for a common item
    Triple,
}

#[derive(Parser)]
struct Args {
    /// Input file path
    #[arg(short, long, default_value = "input.txt")]
    input_file: PathBuf,
    /// What mode to run the program in
    #[arg(value_enum)]
    mode: Mode,
}

/// Find the common item (character) from among `NUM_SACKS` different collections of ascii characters
fn get_common_item<const NUM_SACKS: usize>(sacks: [&[u8]; NUM_SACKS]) -> u8 {
    // Create a copy of each of the sacs so that we can sort them
    let mut sacks = sacks.map(|sack| sack.to_vec());
    for sack in sacks.iter_mut() {
        sack.sort();
    }

    // Create an iterator for each sack to walk through that sack. `sack_tops` are the next item under consideration
    let mut sack_iters = sacks.map(|sack| sack.into_iter());
    let mut sack_tops = sack_iters.clone().map(|mut iter| iter.next().unwrap());

    // Loop through all of the sacks, checking for matching characters. Each loop iterates only one iterator from a sack at a time.
    loop {
        // Go through every item currently under consideration and check for two things:
        // 1) If they're identical, return Ok with the identical value
        // 2) If they aren't identical, return Err with the minimum value and the index of the sack with the minimum value
        // This is done with an accumulation operation by skipping the first item and putting it in as the initial accumulator
        match sack_tops.iter().enumerate().skip(1).fold(
            Ok(sack_tops.first().unwrap()),
            |acc, (i, top)| match acc {
                // If the accumulator is Ok, then every value before this is identical.
                // If the next value is still identical, then return an Ok signaling everything is still identical
                Ok(acc) if acc == top => Ok(acc),
                // Otherwise, switch to Err and record the smaller value
                Ok(acc) if acc < top => Err((i - 1, acc)),
                Ok(_) => Err((i, top)),
                // If the accumulator is Err, then we know something isn't identical and we just need to find the minimum value,
                // so record the smaller one.
                Err((j, min)) if min < top => Err((j, min)),
                Err(_) => Err((i, top)),
            },
        ) {
            // If the accumulation operation returns Ok, then that means everything was identical and we
            // found the common element between the sacks - return it
            Ok(acc) => return *acc,
            // Otherwise, we need to keep searching for the common element. The accumulation returns which sack has the smallest currently considered
            // value, so we iterate that sack and look at the next value. Since all of the sacks are sorted and we only iterate the sack with the
            // smallest considered value, we know that this value can't be common between the sacks.
            Err((i, _)) => {
                if let Some(top) = sack_iters[i].next() {
                    sack_tops[i] = top;
                } else {
                    // If there aren't any more items in the sacks, then we failed to find the common item between the sacks.
                    // Break the loop
                    break;
                }
            }
        }
    }

    // There should always be a common item between the sacks, so this is unreachable
    unreachable!()
}

/// Split a single string into multiple substrings of equal size
fn split_sacks<const NUM_SACKS: usize>(string: &[u8]) -> [&[u8]; NUM_SACKS] {
    let size = string.len() / NUM_SACKS;

    (0..NUM_SACKS)
        .map(|i| &string[(i * size)..((i + 1) * size)])
        // I wish there was a try_collect
        .collect::<Vec<_>>()
        .try_into()
        .unwrap()
}

#[test]
/// Test the `common_items` function with given examples from the page
fn test_common_items() {
    macro_rules! test_first {
        ($exp1:expr, $exp2:expr) => {
            let sacks = split_sacks::<2>($exp1);
            assert_eq!(
                get_common_item(sacks) as char,
                $exp2,
                "Finding similar item in\n  left: `{}`\n right: `{}`",
                String::from_utf8_lossy(sacks[0]),
                String::from_utf8_lossy(sacks[1])
            );
        };
    }
    test_first!(b"vJrwpWtwJgWrhcsFMMfFFhFp", 'p');
    test_first!(b"jqHRNqRjqzjGDLGLrsFMfFZSrLrFZsSL", 'L');
    test_first!(b"PmmdzqPrVvPwwTWBwg", 'P');
    test_first!(b"wMqvLMZHhHMvwLHjbvcjnnSBnvTQFn", 'v');
    test_first!(b"ttgJtRGJQctTZtZT", 't');
    test_first!(b"CrZsJsPPZsGzwwsLwLmpwMDw", 's');

    assert_eq!(
        get_common_item([
            b"vJrwpWtwJgWrhcsFMMfFFhFp",
            b"jqHRNqRjqzjGDLGLrsFMfFZSrLrFZsSL",
            b"PmmdzqPrVvPwwTWBwg"
        ]) as char,
        'r'
    );
    assert_eq!(
        get_common_item([
            b"wMqvLMZHhHMvwLHjbvcjnnSBnvTQFn",
            b"ttgJtRGJQctTZtZT",
            b"CrZsJsPPZsGzwwsLwLmpwMDw"
        ]) as char,
        'Z'
    );
}

/// Convert an item to a priority
fn priority(item: u8) -> u8 {
    if item <= b'Z' {
        item - b'A' + 27
    } else {
        item - b'a' + 1
    }
}

fn main() -> Result<(), Box<dyn Error>> {
    let args = Args::parse();

    let file = File::open(args.input_file)?;

    // Get an iterator over the lines of the input file
    let lines = io::BufReader::new(file)
        .lines()
        // Skip lines which couldn't be read
        .flatten()
        .map(|s| s.into_bytes());

    // Convert the lines into common items (either in halves of a sack or between multiple sacks) depending on mode
    let item_iter: Box<dyn Iterator<Item = _>> = match args.mode {
        Mode::Single => Box::new(lines.map(|sack| get_common_item(split_sacks::<2>(&sack)))),
        Mode::Triple => Box::new(
            lines
                .array_chunks::<3>()
                // Annoying type conversions
                .map(|sacks| get_common_item(sacks.each_ref().map(|v| &v[..]))),
        ),
    };

    // Convert common items into priorities, then sum
    let sum = item_iter.map(|item| priority(item) as u64).sum::<u64>();

    println!("{sum}");

    Ok(())
}
