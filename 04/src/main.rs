#![deny(clippy::pedantic)]
use std::{
	fs::File,
	io::{self, BufRead},
	path::PathBuf,
	str::FromStr,
};

use anyhow::Result;
use clap::{Parser, ValueEnum};
use lazy_static::lazy_static;
use regex::Regex;

#[derive(Clone, ValueEnum)]
enum Mode {
	/// The first variant of the problem, where we check if in a pair of assignments, one overlaps entirely with the other
	Entire,
	/// The second variant of the problem, where we check if in a pair of assignments, one overlaps the other at all
	Partial,
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

/// A pair of section assignments. Each section assignment is a pair of numbers, which represent a range of sections.
struct Assignments((u32, u32), (u32, u32));

impl Assignments {
	/// Test if one assignment fully contains the other
	fn overlaps_entirely(&self) -> bool {
		(self.0 .0 >= self.1 .0 && self.0 .1 <= self.1 .1)
			|| (self.0 .0 <= self.1 .0 && self.0 .1 >= self.1 .1)
	}

	/// Test if the assignments overlap at all
	fn overlaps_partially(&self) -> bool {
		!((self.0 .0 < self.1 .0 && self.0 .1 < self.1 .0)
			|| (self.0 .0 > self.1 .1 && self.0 .1 > self.1 .1))
	}
}

impl FromStr for Assignments {
	type Err = anyhow::Error;

	fn from_str(text: &str) -> std::result::Result<Self, Self::Err> {
		// Lazily initialize a static regular expression for parsing a pair of assignments
		lazy_static! {
			static ref REGEX: Regex =
				Regex::new("^([[:digit:]]+)-([[:digit:]]+),([[:digit:]]+)-([[:digit:]]+)$")
					.unwrap();
		}

		// Each number above is captured in a capture group - use those to parse
		let captures = REGEX.captures(text).unwrap();

		Ok(Assignments(
			(captures[1].parse()?, captures[2].parse()?),
			(captures[3].parse()?, captures[4].parse()?),
		))
	}
}

fn main() -> Result<()> {
	let args = Args::parse();

	let file = File::open(args.input_file)?;

	// Change modes based on which part of the problem
	let overlaps = match args.mode {
		Mode::Entire => Assignments::overlaps_entirely,
		Mode::Partial => Assignments::overlaps_partially,
	};

	let overlaps: u32 = io::BufReader::new(file)
		.lines()
		// Skip lines which couldn't be read
		.flatten()
		// Parse lines as assignment pairs
		.flat_map(|s| s.parse::<Assignments>())
		// Check if assignment pair overlaps - if so, count it (as 1)
		.map(|assignment| u32::from(overlaps(&assignment)))
		// Then sum overlapping assignments
		.sum();

	println!("No. overlapping assignments: {overlaps}");

	Ok(())
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn test_overlaps() {
		macro_rules! test_entirely {
			($str:expr, $truth:expr) => {
				let assignment: Assignments = $str.parse().unwrap();
				let overlaps = assignment.overlaps_entirely();

				assert_eq!(overlaps, $truth, "(entire overlap)\n  text: `{}`", $str)
			};
		}

		test_entirely!("2-4,6-8", false);
		test_entirely!("2-3,4-5", false);
		test_entirely!("5-7,7-9", false);
		test_entirely!("2-8,3-7", true);
		test_entirely!("6-6,4-6", true);
		test_entirely!("2-6,4-8", false);

		macro_rules! test_partially {
			($str:expr, $truth:expr) => {
				let assignment: Assignments = $str.parse().unwrap();
				let overlaps = assignment.overlaps_partially();

				assert_eq!(overlaps, $truth, "(partial overlap)\n  text: `{}`", $str)
			};
		}

		test_partially!("2-4,6-8", false);
		test_partially!("2-3,4-5", false);
		test_partially!("5-7,7-9", true);
		test_partially!("2-8,3-7", true);
		test_partially!("6-6,4-6", true);
		test_partially!("2-6,4-8", true);
	}

	#[test]
	fn test_parse() {
		macro_rules! test {
			($str:expr, $n_tuple:expr) => {
				let assignment: Assignments = $str.parse().unwrap();
				let nums = (
					assignment.0 .0,
					assignment.0 .1,
					assignment.1 .0,
					assignment.1 .1,
				);

				assert_eq!(nums, $n_tuple, "\n  text: `{}`", $str)
			};
		}

		test!("2-4,6-8", (2, 4, 6, 8));
		test!("2-3,4-5", (2, 3, 4, 5));
		test!("5-7,7-9", (5, 7, 7, 9));
		test!("2-8,3-7", (2, 8, 3, 7));
		test!("6-6,4-6", (6, 6, 4, 6));
		test!("2-6,4-8", (2, 6, 4, 8));

		// An extra one to make sure it works with multiple digits (as mentioned in the prompt)
		test!("22-63,4-888", (22, 63, 4, 888));
	}
}
