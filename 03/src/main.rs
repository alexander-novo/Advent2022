#![feature(iter_array_chunks)]
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

// Find the common item from among
fn get_common_item<const NUM_SACKS: usize>(sacks: [&[u8]; NUM_SACKS]) -> u8 {
    let mut sacks = sacks.map(|sack| sack.to_vec());
    for sack in sacks.iter_mut() {
        sack.sort();
    }

    let mut sack_iters = sacks.map(|sack| sack.into_iter());
    let mut sack_tops = sack_iters.clone().map(|mut iter| iter.next().unwrap());

    loop {
        match sack_tops.iter().enumerate().skip(1).fold(
            Ok(sack_tops.first().unwrap()),
            |acc, (i, top)| match acc {
                Ok(acc) if acc == top => Ok(acc),
                Ok(acc) if acc < top => Err((i - 1, acc)),
                Ok(_) => Err((i, top)),
                Err((j, min)) if min < top => Err((j, min)),
                Err(_) => Err((i, top)),
            },
        ) {
            Ok(acc) => return *acc,
            Err((i, _)) => {
                if let Some(top) = sack_iters[i].next() {
                    sack_tops[i] = top;
                } else {
                    break;
                }
            }
        }
    }

    unreachable!()
}

fn split_sacks<const NUM_SACKS: usize>(string: &[u8]) -> [&[u8]; NUM_SACKS] {
    let size = string.len() / NUM_SACKS;

    (0..NUM_SACKS)
        .map(|i| &string[(i * size)..((i + 1) * size)])
        .collect::<Vec<_>>()
        .try_into()
        .unwrap()
}

#[test]
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

    let lines = io::BufReader::new(file)
        .lines()
        // Skip lines which couldn't be read
        .flatten();

    let sum: u64 = match args.mode {
        Mode::Single => lines
            .map(|sack| priority(get_common_item(split_sacks::<2>(sack.as_bytes()))) as u64)
            .sum(),
        Mode::Triple => lines
            .array_chunks::<3>()
            .map(|sacks| {
                priority(get_common_item([
                    sacks[0].as_bytes(),
                    sacks[1].as_bytes(),
                    sacks[2].as_bytes(),
                ])) as u64
            })
            .sum(),
    };

    println!("{sum}");

    Ok(())
}
