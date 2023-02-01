#![feature(try_blocks)]
#![deny(clippy::pedantic)]
use std::{
	error::Error,
	fs::File,
	io::{self, BufRead},
	path::PathBuf,
};

use clap::{Parser, ValueEnum};

#[derive(Clone, ValueEnum)]
enum Mode {
	/// The first variant of the problem, where the second letter in each line of the file tells you what shape to put your hand in
	Shape,
	/// The second variant of the problem, where the second letter in each line of the file tells you how you should win
	Win,
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

/// The first version of scoring, where the second player's input is the shape they should make.
/// `p` is the tuple of player inputs, corresponding to these:
/// 0 - Rock, 1 - Paper, 2 - Scissors
fn score_shape(p1: u8, p2: u8) -> u8 {
	// Part of scoring solely based on shape
	(p2 + 1)
	// Then calculate who won. Note how each number beats the one before it. Then we can take the difference
	// and use it to calculate the winner. If they're the same, then the difference is 0 and it's a tie. If the difference is 1,
	// Then player 1 won and we lost, and if the difference is -1 (2 in euclidean division), then we won
        + match (i16::from(p1) - i16::from(p2)).rem_euclid(3) {
            0 => 3,
            1 => 0,
            2 => 6,
            _ => unreachable!(),
        }
}

/// The second version of scoring, where the second player's input is how they should win.
/// `p` is the tuple of player inputs, where player 1's inputs are as above in [`score_shape`], and player 2's inputs are:
/// 0 - lose, 1 - tie, 2 - win
fn score_win(p1: u8, p2: u8) -> u8 {
	let re: Result<u8, anyhow::Error> = try {
		// This is the scoring based on win
		p2 * 3
			// What shape we should play to win, Uses inverse logic as in score_shape above - if we want to lose, simply subtract 1,
			// if we want to tie, do nothing ,and if we want to win, add 1 (then wrap as necessary)
			+ u8::try_from((i8::try_from(p1)? + (i8::try_from(p2)? - 1)).rem_euclid(3) + 1)?
	};

	re.unwrap()
}

fn main() -> Result<(), Box<dyn Error>> {
	let args = Args::parse();

	// Load input file, make sure it's openable
	let file = File::open(args.input_file)?;

	// Switch the scoring mode based on arguments
	let score = match args.mode {
		Mode::Shape => score_shape,
		Mode::Win => score_win,
	};

	// Read lines from file
	let total_score: u32 = io::BufReader::new(file)
		.lines()
		// Skip lines which couldn't be read
		.flatten()
		// Convert letters into 0-based inputs as expected by score_ functions,
		// and then convert to scores depending on chosen scoring method
		.map(|s| {
			let b = s.as_bytes();
			u32::from(score(b[0] - b'A', b[2] - b'X'))
		})
		// Then sum up the scores
		.sum();

	println!("{total_score}");

	Ok(())
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn test_shape() {
		// Tests given by page
		assert_eq!(score_shape(b'A' - b'A', b'Y' - b'X'), 8);
		assert_eq!(score_shape(b'B' - b'A', b'X' - b'X'), 1);
		assert_eq!(score_shape(b'C' - b'A', b'Z' - b'X'), 6);
	}

	#[test]
	fn test_win() {
		// Tests given by page
		assert_eq!(score_win(b'A' - b'A', b'Y' - b'X'), 4);
		assert_eq!(score_win(b'B' - b'A', b'X' - b'X'), 1);
		assert_eq!(score_win(b'C' - b'A', b'Z' - b'X'), 7);
	}
}
