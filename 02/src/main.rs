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
        + match (p1 as i8 - p2 as i8).rem_euclid(3) {
            0 => 3,
            1 => 0,
            2 => 6,
            _ => unreachable!(),
        }
}

#[test]
// Tests given by page
fn test_shape() {
	assert_eq!(score_shape(b'A' - b'A', b'Y' - b'X'), 8);
	assert_eq!(score_shape(b'B' - b'A', b'X' - b'X'), 1);
	assert_eq!(score_shape(b'C' - b'A', b'Z' - b'X'), 6);
}

/// The second version of scoring, where the second player's input is how they should win.
/// `p` is the tuple of player inputs, where player 1's inputs are as above in [`score_shape`], and player 2's inputs are:
/// 0 - lose, 1 - tie, 2 - win
fn score_win(p1: u8, p2: u8) -> u8 {
	// This is the scoring based on win
	p2 * 3
	// What shape we should play to win, Uses inverse logic as in score_shape above - if we want to lose, simply subtract 1,
	// if we want to tie, do nothing ,and if we want to win, add 1 (then wrap as necessary)
	+ ((p1 as i8 + (p2 as i8 - 1)).rem_euclid(3) + 1) as u8
}

#[test]
// Tests given by page
fn test_win() {
	assert_eq!(score_win(b'A' - b'A', b'Y' - b'X'), 4);
	assert_eq!(score_win(b'B' - b'A', b'X' - b'X'), 1);
	assert_eq!(score_win(b'C' - b'A', b'Z' - b'X'), 7);
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
			score(b[0] - b'A', b[2] - b'X') as u32
		})
		// Then sum up the scores
		.sum();

	println!("{total_score}");

	Ok(())
}
