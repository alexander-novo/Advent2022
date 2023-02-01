#![feature(get_many_mut)]
#![deny(clippy::pedantic)]
use std::{
	collections::VecDeque,
	fs::File,
	io::{self, BufRead},
	path::{Path, PathBuf},
	str::FromStr,
	time::Duration,
};

use anyhow::Result;
use clap::{Parser, ValueEnum};
use indicatif::{ProgressBar, ProgressStyle};
use lazy_static::lazy_static;
use regex::Regex;

#[derive(Clone, ValueEnum)]
enum Mode {
	/// The first variant of the problem, with CrateMover 9000, who inverts the stacks of crates onto other stacks
	Reverse,
	/// The second variant of the problem, with CreateMover 9001, who takes stacks of crates as-is and moves them onto other stacks
	NoReverse,
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

/// Do a cursory parse through the lines of the input file, and find out the number of stacks,
/// the largest initial size of a stack, and how many commands there will be to process.
/// Assumes at most 9 stacks.
fn get_num_stacks_and_stack_size<T: Iterator<Item = String>>(
	mut lines: T,
) -> (usize, usize, usize) {
	// Figure out how many stacks there are and a good initial size for the stacks
	// by first finding the bottom line of the initial stack setup. This line
	// tells us how many stacks there are, and how many lines before it tells us how large
	// these stacks need to be to fit the initial setup.
	let mut num_stacks = 0;
	let stack_size = lines
		.by_ref()
		.take_while(|line| {
			if line.starts_with(" 1") {
				num_stacks = line.bytes().skip(1).step_by(4).count();
				false
			} else {
				true
			}
		})
		.count();

	// The remaining lines (except for a blank one) are all commands to process
	let num_commands = lines.skip(1).count();

	(num_stacks, stack_size, num_commands)
}

/// Parse the first half of the input file into stacks
fn get_initial_stacks<T: Iterator<Item = String>>(
	lines: &mut T,
	num_stacks: usize,
	stack_size: usize,
) -> Vec<VecDeque<u8>> {
	// Create our stacks
	let mut stacks: Vec<_> = vec![VecDeque::with_capacity(stack_size); num_stacks];

	// Add on to the stacks for each line in the initial stack setup
	lines
		// We want to continue reading lines after reading the initial setup of the stacks, so take by reference
		.by_ref()
		// Read the stack setup. We know how many lines there are here, because we counted them in stack_size
		.take(stack_size)
		// For each line, add the crate contents to the corresponding stack
		.for_each(|line| {
			let contents = line.bytes().skip(1).step_by(4);

			stacks
				.iter_mut()
				.zip(contents)
				// Only add contents (not blank spaces) to the stacks
				.filter(|(_stack, c)| *c != b' ')
				.for_each(|(stack, c)| {
					// Using push_front here because we're reading top-down
					// and later we can do normal stack operations
					stack.push_front(c);
				});
		});

	stacks
}

#[derive(Debug)]
/// Struct epresenting a single move command a la 'move 1 from 2 to 1'
struct Command {
	/// How many crates to move
	num_moved: usize,
	/// Which stack to move from
	stack_from: usize,
	/// Which stack to move to
	stack_to: usize,
}

impl FromStr for Command {
	type Err = anyhow::Error;

	fn from_str(text: &str) -> std::result::Result<Self, Self::Err> {
		// Lazily initialize a static regular expression for parsing a command
		lazy_static! {
			static ref REGEX: Regex =
				Regex::new("^move ([[:digit:]]+) from ([[:digit:]]) to ([[:digit:]])$").unwrap();
		}

		// Each number above is captured in a capture group - use those to parse
		let captures = REGEX
			.captures(text)
			.unwrap_or_else(|| panic!("Command `{text}` doesn't match regex"));

		Ok(Command {
			num_moved: captures[1].parse()?,
			stack_from: captures[2].parse::<usize>()? - 1,
			stack_to: captures[3].parse::<usize>()? - 1,
		})
	}
}

fn simulate<const REVERSE: bool, T: Iterator<Item = String>>(
	lines: T,
	mut stacks: Vec<VecDeque<u8>>,
) -> impl Iterator<Item = u8> {
	lines
		.flat_map(|line| line.parse::<Command>())
		.for_each(|command| {
			let stack_from = &mut stacks[command.stack_from];
			let mut temp = stack_from.split_off(stack_from.len() - command.num_moved);

			if REVERSE {
				temp.make_contiguous().reverse();
			}

			let stack_to = &mut stacks[command.stack_to];
			stack_to.append(&mut temp);
		});

	stacks.into_iter().map(|stack| *stack.back().unwrap())
}

fn lines_reader<P: AsRef<Path>>(p: P) -> Result<impl Iterator<Item = String>> {
	let file = File::open(p)?;
	Ok(io::BufReader::with_capacity(10_000_000, file)
		.lines()
		// Skip lines which couldn't be read
		.flatten())
}

fn main() -> Result<()> {
	let args = Args::parse();

	let lines = lines_reader(&args.input_file)?;
	let (num_stacks, stack_size, num_commands) = get_num_stacks_and_stack_size(lines);

	let mut lines = lines_reader(&args.input_file)?;
	let stacks = get_initial_stacks(&mut lines, num_stacks, stack_size);

	// Skip the number line and blank line in the instructions
	let lines = lines.skip(2);

	// Progress bar
	let pb =
		ProgressBar::new(num_commands as u64)
			.with_style(
				ProgressStyle::with_template("{spinner:.green} [{elapsed_precise}] [{wide_bar:.cyan/blue}] {human_pos}/{human_len} ({eta})")
					.unwrap()
					.progress_chars("#>-")
			);
	// Don't update progress bar every time we simulate a command. Instead do it every .1 second.
	pb.enable_steady_tick(Duration::from_millis(100));

	// Add progress bar to iterator
	let lines = pb.wrap_iter(lines);

	let tops = match args.mode {
		Mode::Reverse => simulate::<true, _>(lines, stacks).collect::<Vec<_>>(),
		Mode::NoReverse => simulate::<false, _>(lines, stacks).collect::<Vec<_>>(),
	};
	let top = String::from_utf8_lossy(&tops);

	println!("{top}");

	Ok(())
}

#[cfg(test)]
mod tests {
	use super::*;

	// Example given in prompt
	static EXAMPLE: &str = "    [D]    
[N] [C]    
[Z] [M] [P]
 1   2   3 
 
move 1 from 2 to 1
move 3 from 1 to 3
move 2 from 2 to 1
move 1 from 1 to 2";

	#[test]
	fn command_parse() {
		macro_rules! test {
			($str:expr, $tuple:expr) => {
				let command = $str.parse::<Command>().unwrap();
				let command = (
					command.num_moved,
					command.stack_from + 1,
					command.stack_to + 1,
				);

				assert_eq!(command, $tuple);
			};
		}

		test!("move 1 from 2 to 1", (1, 2, 1));
		test!("move 3 from 1 to 3", (3, 1, 3));
		test!("move 2 from 2 to 1", (2, 2, 1));
		test!("move 1 from 1 to 2", (1, 1, 2));
	}

	#[test]
	fn initial_stacks() {
		let lines: Vec<_> = EXAMPLE
			.lines()
			.map(std::string::ToString::to_string)
			.collect();

		let (num_stacks, stack_size, num_commands) =
			get_num_stacks_and_stack_size(lines.clone().into_iter());

		let mut lines = lines.into_iter();
		let mut stacks = get_initial_stacks(&mut lines, num_stacks, stack_size);

		assert_eq!(num_stacks, 3);
		assert_eq!(stack_size, 3);
		assert_eq!(num_commands, 4);

		macro_rules! test_stack {
			($idx:expr, $str:expr) => {
				assert_eq!(
					String::from_utf8_lossy(stacks[$idx - 1].make_contiguous()),
					$str
				);
			};
		}
		test_stack!(1, "ZN");
		test_stack!(2, "MCD");
		test_stack!(3, "P");
	}

	#[test]
	fn test_simulate() {
		let lines: Vec<_> = EXAMPLE
			.lines()
			.map(std::string::ToString::to_string)
			.collect();

		let (num_stacks, stack_size, _num_commands) =
			get_num_stacks_and_stack_size(lines.clone().into_iter());

		let mut lines = lines.into_iter();
		let stacks = get_initial_stacks(&mut lines, num_stacks, stack_size);

		// Skip the number line and blank line in the instructions
		let lines = lines.skip(2);

		let tops = simulate::<true, _>(lines.clone(), stacks.clone()).collect::<Vec<_>>();
		let top = String::from_utf8_lossy(&tops);

		assert_eq!(top, "CMZ");

		let tops = simulate::<false, _>(lines, stacks).collect::<Vec<_>>();
		let top = String::from_utf8_lossy(&tops);

		assert_eq!(top, "MCD");
	}
}
