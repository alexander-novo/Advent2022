#![deny(clippy::pedantic)]
use std::{
	fs::File,
	io::{self, BufRead},
	path::PathBuf,
	str::FromStr,
};

use anyhow::{anyhow, Result};
use clap::{Parser, ValueEnum};
use lazy_static::lazy_static;
use regex::Regex;

#[derive(Clone, ValueEnum)]
enum Mode {
	/// The first variant of the problem, where we find the size of all directories below a certain size (100,000)
	SmallDirSize,
	/// The second variant of the problem, where we find the size of the smallest directory we can delete which will give us enough free space
	FreeSpace,
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

/// An enum which keeps track of listings that actually matter:
/// - `cd ..`   (traversing to a parent directory)
/// - `cd a`    (traversing to a child directory (here named 'a'))
/// - `29116 f` (the size of a file in a directory)
///
/// ls and dir listings not represented because they don't provide any meaningful information
enum Listing {
	ChangeDirDown,
	ChangeDirUp,
	File(u64),
}

impl FromStr for Listing {
	type Err = anyhow::Error;

	fn from_str(s: &str) -> std::result::Result<Self, Self::Err> {
		// A regex for parsing the important listings.
		// Each variant has its own named capture group that captures relevant information,
		// such as file size in a file listing. So we can use these capture groups to determine
		// the variant after matching a single time.
		lazy_static! {
			static ref REGEX: Regex = Regex::new(
				r"^(?:\$ cd (?:(?P<dir_up>\.\.)|(?P<dir_down>\S+)))|(?:(?P<file_size>\d+) \S+)$"
			)
			.unwrap();
		}

		match REGEX.captures(s) {
			Some(captures) => {
				if captures.name("dir_up").is_some() {
					Ok(Listing::ChangeDirUp)
				} else if captures.name("dir_down").is_some() {
					Ok(Listing::ChangeDirDown)
				} else if let Some(size) = captures.name("file_size") {
					Ok(Listing::File(size.as_str().parse()?))
				}
				// If we matched, we should have matched one of those capture groups
				else {
					Err(anyhow!(
						"Found a meaningful listing, but couldn't get the capture groups to work"
					))
				}
			}
			// If we didn't match, then the listing is irrelevant, like ls or dir
			None => Err(anyhow!("Couldn't find a meaningful listing")),
		}
	}
}

/// Finds the total size of all directories below a certain max size (100,000)
/// from a list of commands navigating directories.
fn total_size<T: Iterator<Item = String>>(lines: T) -> u64 {
	const MAX_SIZE: u64 = 100_000;
	// The total which we will return later
	let mut sum = 0;

	// A list of sizes of a directory and all of its parent directories, in reverse order
	let mut dir_sizes = Vec::new();

	lines
		// Parse each line
		.flat_map(|line| line.parse::<Listing>())
		.for_each(|listing| match listing {
			// If we're going down in directories (such as with `cd a`), add a new empty directory
			Listing::ChangeDirDown => dir_sizes.push(0),
			// If we're going up in directories (such as with `cd ..`), pop this directory off,
			// and add it to the sum if it's under MAX_SIZE
			Listing::ChangeDirUp => {
				let size = dir_sizes.pop().unwrap();

				// Each directory above this one also has the size of this directory
				let upper_size = dir_sizes.last_mut().unwrap();
				*upper_size += size;

				if size <= MAX_SIZE {
					sum += size;
				}
			}
			// Otherwise, if we're looking at a file entry, add its size to the current directory
			Listing::File(size) => *dir_sizes.last_mut().unwrap() += size,
		});

	// Once we're done with all of the listings, we're left with a bunch of directories which
	// were never navigated out of, so we need to process them to.
	sum += dir_sizes
		.iter()
		// Reverse order because we've been using dir_sizes like a stack
		.rev()
		// Because we never popped these directories out, they never had the sizes of their last child
		// added to their size (see above), so we need to do that here. acc keeps track of the size of the
		// last child, and starts at 0 since this last directory has no unprocessed children
		.scan(0, |acc, size| {
			*acc += size;

			if *acc <= MAX_SIZE {
				Some(*acc)
			} else {
				None
			}
		})
		// Then add these directories to the sum
		.sum::<u64>();

	sum
}

fn smallest_deletable_dir<T: Iterator<Item = String>>(lines: T) -> u64 {
	// The total space on the drive
	const TOTAL_SPACE: u64 = 70_000_000;
	// How much free space we want to end up with
	const FREE_SPACE: u64 = 30_000_000;
	// A list of sizes of a directory and all of its parent directories, in reverse order
	let mut dir_sizes = Vec::new();

	// A list of sizes of all directories in post-order traversal order
	let mut all_dir_sizes = Vec::new();

	// The same as above in total_size, except that instead of summing sizes,
	// we push them in all_dir_sizes to be processed later.
	lines
		.flat_map(|line| line.parse::<Listing>())
		.for_each(|listing| match listing {
			Listing::ChangeDirDown => dir_sizes.push(0),
			Listing::ChangeDirUp => {
				let size = dir_sizes.pop().unwrap();

				let upper_size = dir_sizes.last_mut().unwrap();

				*upper_size += size;

				all_dir_sizes.push(size);
			}
			Listing::File(size) => *dir_sizes.last_mut().unwrap() += size,
		});

	// Similarly to above, we need to process the remaining leftover directories we didn't back
	// out of at the end of the listings. We'll add those on to the end of all_dir_sizes
	all_dir_sizes.extend(dir_sizes.iter().rev().scan(0, |acc, size| {
		*acc += size;

		Some(*acc)
	}));

	// The total size everything is taking up is the size of the / directory, which should be the last directory
	// in all_dir_sizes since it is in post-order traversal order
	let total_size = all_dir_sizes.last().unwrap();
	// The minimum amount of space we need to free - which is our goal free space minus our current free space
	let goal_size = FREE_SPACE - (TOTAL_SPACE - total_size);

	// Then find the smallest directory whose size exceeds the goal_size
	all_dir_sizes
		.into_iter()
		.filter(|size| *size >= goal_size)
		.min()
		.unwrap()
}

fn main() -> Result<()> {
	let args = Args::parse();

	let file = File::open(args.input_file)?;
	let lines = io::BufReader::with_capacity(10_000_000, file)
		.lines()
		// Skip lines which couldn't be read
		.flatten();

	let size = match args.mode {
		Mode::SmallDirSize => total_size(lines),
		Mode::FreeSpace => smallest_deletable_dir(lines),
	};

	println!("{size}");

	Ok(())
}

#[cfg(test)]
mod tests {
	use super::*;

	// The example prompt
	static PROMPT: &str = "$ cd /
$ ls
dir a
14848514 b.txt
8504156 c.dat
dir d
$ cd a
$ ls
dir e
29116 f
2557 g
62596 h.lst
$ cd e
$ ls
584 i
$ cd ..
$ cd ..
$ cd d
$ ls
4060174 j
8033020 d.log
5626152 d.ext
7214296 k";

	#[test]
	fn example() {
		let lines = PROMPT.lines().map(std::string::ToString::to_string);

		assert_eq!(total_size(lines.clone()), 95437);
		assert_eq!(smallest_deletable_dir(lines), 24_933_642);
	}
}
