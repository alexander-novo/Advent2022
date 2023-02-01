#![deny(clippy::pedantic)]
use std::{
	cmp::Reverse,
	collections::BinaryHeap,
	error::Error,
	fs::File,
	io::{self, BufRead},
	path::PathBuf,
};

use clap::Parser;
use itertools::Itertools;

#[derive(Parser)]
struct Args {
	/// Input file path
	#[arg(short, long, default_value = "input.txt")]
	input_file: PathBuf,
	/// The number of elves to find with the maximum amount of calories.
	/// Change to 1 for part 1 of the problem
	#[arg(short, long, default_value_t = 3)]
	num_elves: usize,
}

fn main() -> Result<(), Box<dyn Error>> {
	let args = Args::parse();

	// Load input file, make sure it's openable
	let file = File::open(args.input_file)?;

	// Start reading file use a buffered reader
	let mut calorie_iter = io::BufReader::new(file)
		// Read by lines. Each line is either a single calorie number, or a separator (blank)
		.lines()
		// Reading a line can fail due to non-unicode characters being present in that line, so lines() returns an iterator over results of strings.
		// I don't care about lines that have failed to read, so I skip them by flattening the iterator and end up with an iterator over just strings.
		.flatten()
		// Convert each line to a number. Blank separator lines will fail to parse, separating the iterator into runs of Ok(u32) snacks separated by Err(...) for each elf
		.map(|l| l.parse::<u32>())
		// Sum the runs of Ok(u32) into single Ok(u32) containing total calories for each elf alternating with Err(...)
		.coalesce(|x, y| match (&x, &y) {
			(Ok(x), Ok(y)) => Ok(Ok(x + y)),
			_ => Err((x, y)),
		})
		// Get rid of the Err(...) separators. Now we just have an iterator over total calories by elf.
		.flatten()
		// Convenience for min-heap
		.map(Reverse);

	// Initialize a min-heap which keeps track of the n most total calories per elf, starting with the first n elves.
	let mut heap = calorie_iter
		.by_ref()
		.take(args.num_elves)
		.collect::<BinaryHeap<_>>();

	// Then for each remaining elf, check to see if their total calories are one of the top n calories we've seen so far
	// by comparing them to the numbers we've stored in the heap. A new calorie value will be one of the top n values we've seen so far
	// if it's greater than *any* of the top n values we've previously seen. And if it's greater than *any* of them, it must be greater than
	// the smallest one, which is stored on top of the min-heap. In that case, remove the smallest one and add the new value into the heap.
	// Note the heap stores Reverse(x) so that it can be a min-heap, so the comparison is backwards.
	calorie_iter.for_each(|x| {
		if heap.peek().unwrap() > &x {
			heap.pop();
			heap.push(x);
		}
	});

	// Then once we've found the top n total calories per elf, sum them up and we have an answer
	let calories: u32 = heap.into_iter().map(|x| x.0).sum();

	println!(
		"Calories carried by the top {} elves: {calories}",
		args.num_elves
	);

	Ok(())
}
