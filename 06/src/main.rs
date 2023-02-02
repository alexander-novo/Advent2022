#![feature(iter_array_chunks)]
#![deny(clippy::pedantic)]
use std::{collections::VecDeque, path::PathBuf};

use anyhow::Result;
use clap::{Parser, ValueEnum};

#[derive(Clone, ValueEnum)]
enum Mode {
	/// The first variant of the problem, where we find the start-of-packet marker, which is a window of 4 unique characters
	Packet,
	/// The second variant of the problem, where we find the start-of-message marker, which is a window of 14 unique characters
	Message,
}

impl Mode {
	const fn window_size(&self) -> usize {
		match self {
			Mode::Packet => 4,
			Mode::Message => 14,
		}
	}
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

/// Converts a u8 representing one lowercase ascii letter of the alphabet to a single u32,
/// with a single bit set to 1. There are 26 such characters and 32 available bits, so each one is unique.
fn convert_bits(c: u8) -> u32 {
	1 << (c - b'a')
}

fn find_start_of_packet<const WINDOW_SIZE: usize>(string: &str) -> usize {
	let mut iter = string.as_bytes().iter().map(|c| convert_bits(*c));
	// A queue for remembering which items are currently being considered in the window.
	// Space for WINDOW_SIZE + 1 items instead of WINDOW_SIZE, since it's easier if there's room for the next item
	// in the next window before pushing out the previous item.
	let mut window = VecDeque::with_capacity(WINDOW_SIZE + 1);
	window.extend(iter.clone().take(WINDOW_SIZE));

	// A checksum value which can be used to keep track of the number of unique items in the window.
	// We initialize it to be the XOR of all of the items in the first window
	let checksum = iter
		.by_ref()
		.take(WINDOW_SIZE)
		.reduce(|acc, c| acc ^ c)
		.unwrap();

	// Out first checksum was already calculated, so the iterator should start with that one
	let (i, _) = std::iter::once(checksum)
		// Then after the first checksum, we calculate progressive checksums by popping out the
		// last item from the previous window, XORing it with the previous checksum (therefore removing it since X ^ c ^ X = c),
		// and XORing in the item newly added to the window.
		.chain(iter.scan(checksum, |checksum, c| {
			window.push_back(c);
			let remove = window.pop_front().unwrap();
			*checksum ^= remove ^ c;
			Some(*checksum)
		}))
		// Enumerate so we can find the index of the correct checksum
		.enumerate()
		// The correct checksum is the one with a number of ones set equal to the number of items in the window
		.find(|(_, checksum)| checksum.count_ones() == (WINDOW_SIZE.try_into().unwrap()))
		.unwrap();

	// We had to consume a window of characters to get the first checksum, so add the window size to the return value
	i + WINDOW_SIZE
}

fn main() -> Result<()> {
	let args = Args::parse();

	let communication = std::fs::read_to_string(args.input_file)?;
	let packet_start = match args.mode {
		Mode::Packet => find_start_of_packet::<4>(&communication),
		Mode::Message => find_start_of_packet::<14>(&communication),
	};

	// packet_start is the number of characters which had to be consumed to find the packet start.
	// This means it is the index of the last character in the window
	println!(
		"{}",
		&communication[(packet_start - args.mode.window_size())..packet_start]
	);
	println!("{packet_start}");

	Ok(())
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn start_of_packet() {
		assert_eq!(find_start_of_packet::<4>("bvwbjplbgvbhsrlpgdmjqwftvncz"), 5);
		assert_eq!(find_start_of_packet::<4>("nppdvjthqldpwncqszvftbrmjlhg"), 6);
		assert_eq!(
			find_start_of_packet::<4>("nznrnfrfntjfmvfwmzdfjlvtqnbhcprsg"),
			10
		);
		assert_eq!(
			find_start_of_packet::<4>("zcfzfwzzqfrljwzlrfnpqdbhtmscgvjw"),
			11
		);

		// Second part
		assert_eq!(
			find_start_of_packet::<14>("mjqjpqmgbljsphdztnvjfqwrcgsmlb"),
			19
		);
		assert_eq!(
			find_start_of_packet::<14>("bvwbjplbgvbhsrlpgdmjqwftvncz"),
			23
		);
		assert_eq!(
			find_start_of_packet::<14>("nppdvjthqldpwncqszvftbrmjlhg"),
			23
		);
		assert_eq!(
			find_start_of_packet::<14>("nznrnfrfntjfmvfwmzdfjlvtqnbhcprsg"),
			29
		);
		assert_eq!(
			find_start_of_packet::<14>("zcfzfwzzqfrljwzlrfnpqdbhtmscgvjw"),
			26
		);
	}
}
