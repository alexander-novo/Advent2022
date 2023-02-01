#![feature(get_many_mut)]
use std::{
	collections::VecDeque,
	fs::File,
	io::{self, BufRead},
	path::{Path, PathBuf},
	str::FromStr,
};

use anyhow::Result;
use clap::{Parser, ValueEnum};
use lazy_static::lazy_static;
use regex::Regex;

#[derive(Parser)]
struct Args {
	/// Input file path
	#[arg(short, long, default_value = "input.txt")]
	input_file: PathBuf,
}

fn get_num_stacks_and_stack_size<T: Iterator<Item = String>>(lines: T) -> (usize, usize) {
	// Figure out how many stacks there are and a good initial size for the stacks
	// by first finding the bottom line of the initial stack setup. This line
	// tells us how many stacks there are, and how many lines before it tells us how large
	// these stacks need to be to fit the initial setup.
	let mut num_stacks = 0;
	let stack_size = lines
		.take_while(|line| {
			if !line.starts_with(" 1") {
				true
			} else {
				num_stacks = line.bytes().skip(1).step_by(4).count();
				false
			}
		})
		.count();

	(num_stacks, stack_size)
}

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
					stack.push_front(c)
				});
		});

	stacks
}

#[test]
fn test_initial_stacks() {
	// Example given in prompt
	let setup_string = "    [D]    
[N] [C]    
[Z] [M] [P]
 1   2   3 
 
move 1 from 2 to 1
move 3 from 1 to 3
move 2 from 2 to 1
move 1 from 1 to 2";
	let lines: Vec<_> = setup_string.lines().map(|line| line.to_string()).collect();

	let (num_stacks, stack_size) = get_num_stacks_and_stack_size(lines.clone().into_iter());

	let mut lines = lines.into_iter();
	let mut stacks = get_initial_stacks(&mut lines, num_stacks, stack_size);

	assert_eq!(num_stacks, 3);
	assert_eq!(stack_size, 3);

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

#[derive(Debug)]
struct Command {
	num_moved: usize,
	stack_from: usize,
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

#[test]
fn test_command_parse() {
	macro_rules! test {
		($str:expr, $tuple:expr) => {
			let command: Command = $str.parse().unwrap();
			let command = (command.num_moved, command.stack_from, command.stack_to);

			assert_eq!(command, $tuple);
		};
	}

	test!("move 1 from 2 to 1", (1, 2, 1));
	test!("move 3 from 1 to 3", (3, 1, 3));
	test!("move 2 from 2 to 1", (2, 2, 1));
	test!("move 1 from 1 to 2", (1, 1, 2));
}

fn simulate<const REVERSE: bool, T: Iterator<Item = String>>(
	lines: T,
	mut stacks: Vec<VecDeque<u8>>,
) -> impl Iterator<Item = u8> {
	let mut reverse_stack = VecDeque::with_capacity(if REVERSE {
		stacks.first().unwrap().capacity()
	} else {
		0
	});
	lines
		.flat_map(|line| line.parse::<Command>())
		.for_each(|command| {
			if let Ok([stack_from, mut stack_to]) =
				stacks[..].get_many_mut([command.stack_from, command.stack_to])
			{
				let mut stack_final = &mut reverse_stack;
				if REVERSE {
					std::mem::swap(&mut stack_final, &mut stack_to);
				}
				for _ in 0..command.num_moved {
					stack_to.push_back(stack_from.pop_back().unwrap_or_else(
						|| panic!("Tried to pop out of stack, but stack was empty. Command:\n{command:#?}. Stacks:\nFrom: `{}`\n  To: `{}`",
							String::from_utf8_lossy(stack_from.clone().make_contiguous()),
							String::from_utf8_lossy(stack_to.clone().make_contiguous())
						)
					));
				}

				if REVERSE {
					for _ in 0..command.num_moved {
						stack_final.push_back(stack_to.pop_back().unwrap());
					}
				}
			}
		});

	stacks.into_iter().map(|stack| *stack.back().unwrap())
}

#[test]
fn test_simulate() {
	// Example given in prompt
	let setup_string = "    [D]    
[N] [C]    
[Z] [M] [P]
 1   2   3 
 
move 1 from 2 to 1
move 3 from 1 to 3
move 2 from 2 to 1
move 1 from 1 to 2";

	let lines: Vec<_> = setup_string.lines().map(|line| line.to_string()).collect();

	let (num_stacks, stack_size) = get_num_stacks_and_stack_size(lines.clone().into_iter());

	let mut lines = lines.into_iter();
	let stacks = get_initial_stacks(&mut lines, num_stacks, stack_size);

	// Skip the number line and blank line in the instructions
	let lines = lines.skip(2);

	let tops = simulate::<false, _>(lines.clone(), stacks.clone()).collect::<Vec<_>>();
	let top = String::from_utf8_lossy(&tops);

	assert_eq!(top, "CMZ");

	let tops = simulate::<true, _>(lines, stacks).collect::<Vec<_>>();
	let top = String::from_utf8_lossy(&tops);

	assert_eq!(top, "MCD");
}

fn lines_reader<P: AsRef<Path>>(p: P) -> Result<impl Iterator<Item = String>> {
	let file = File::open(p)?;
	Ok(io::BufReader::new(file)
		.lines()
		// Skip lines which couldn't be read
		.flatten())
}

fn main() -> Result<()> {
	let args = Args::parse();

	let lines = lines_reader(&args.input_file)?;
	let (num_stacks, stack_size) = get_num_stacks_and_stack_size(lines);

	let mut lines = lines_reader(&args.input_file)?;
	let stacks = get_initial_stacks(&mut lines, num_stacks, stack_size);

	// Skip the number line and blank line in the instructions
	let lines = lines.skip(2);

	let tops = simulate::<true, _>(lines, stacks).collect::<Vec<_>>();
	let top = String::from_utf8_lossy(&tops);

	println!("{top}");

	Ok(())
}
