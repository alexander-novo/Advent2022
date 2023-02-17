#![deny(clippy::pedantic)]
#![feature(get_many_mut)]
#![feature(let_chains)]
use std::{fs::File, io::Read, path::PathBuf, str::FromStr};

use anyhow::{anyhow, Result};
use clap::{Parser, ValueEnum};

#[derive(Clone, ValueEnum)]
enum Mode {
	/// The first variant of the problem, where we find the number of trees which are visible from an edge of the forest
	NumVisible,
	/// The second variant of the problem, wher we find the highest scenic score possible out of all the trees.
	ScenicScore,
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

struct TreeGrid {
	heights: Vec<u8>,
	width: usize,
}

impl FromStr for TreeGrid {
	type Err = anyhow::Error;

	fn from_str(s: &str) -> std::result::Result<Self, Self::Err> {
		let width = s.lines().next().ok_or_else(|| anyhow!("No lines"))?.len();
		Ok(TreeGrid {
			heights: s
				.as_bytes()
				.iter()
				.filter(|c| !c.is_ascii_whitespace())
				.map(|c| {
					c.checked_sub(b'0')
						.unwrap_or_else(|| panic!("Couldn't subtract from {}", *c as char))
				})
				.collect(),
			width,
		})
	}
}

mod part1 {
	use super::TreeGrid;
	pub(super) fn visible_trees(tree_grid: &TreeGrid) -> usize {
		// Convert the tree grid to a grid of visibilities -
		// 3-tuples that indicate if a tree in the grid is visible, and the
		// tallest tree in each of two directions. We'll fill the heights in as
		// we go, so by default the tallest trees we know about are the trees in
		// those positions
		let mut first_pass = tree_grid
			.heights
			.iter()
			.map(|height| (false, *height, *height))
			.collect::<Vec<_>>();

		// Now calculate, for each tree, if it is visible from the top or left sides.
		// We must loop through indices rather than the vector itself because we must
		// access a window of trees at a time - and LendingIterator doesn't exist yet.
		(0..first_pass.len()).for_each(|i| {
			// If this tree is on an edge, it is visible
			if i == 0
				|| i == first_pass.len()
				|| i < tree_grid.width
				|| i + tree_grid.width > first_pass.len()
				|| (i % tree_grid.width) == 0
				|| (i % tree_grid.width) == tree_grid.width - 1
			{
				first_pass[i].0 = true;
			} else {
				// Otherwise grab info about the trees above this tree and to the left. These necessarily exist because
				// this tree is not on the edge.
				let [above, left, this] = first_pass
					.get_many_mut([i - tree_grid.width, i - 1, i])
					.unwrap();

				// Due to how we constructed first_pass, this tuple contains the height of the tree under consideration.
				let height = this.1;

				// Due to how we are iterating over first_pass, above and left have already been iterated over once,
				// so their 1,2 tuple values contain the tree of greatest height in the above, left directions, respectively.
				// So we can see this tree if its height is either greater than the height of any tree in the above or left directions.
				this.0 |= height > above.1 || height > left.2;
				// Then record the (potentially) new greatest height of trees in the above/left directions
				this.1 = height.max(above.1);
				this.2 = height.max(left.2);
			}
		});

		// Next we are going to do the right, bottom edges, but first_pass contains a bunch of height information
		// about trees in the above, left directions, so reset them to what we know about (the height of the tree in
		// each grid position).
		first_pass
			.iter_mut()
			.zip(tree_grid.heights.iter())
			.for_each(|(pass, height)| {
				pass.1 = *height;
				pass.2 = *height;
			});

		// Same as above, but now the right,bottom edges.
		// We reverse iteration to preserve the property that when we iterate over a tree, its right,bottom neighbor trees
		// have already been iterated over and processed.
		(0..first_pass.len()).rev().for_each(|i| {
			if i == 0
				|| i == first_pass.len()
				|| i < tree_grid.width
				|| i + tree_grid.width > first_pass.len()
				|| (i % tree_grid.width) == 0
				|| (i % tree_grid.width) == tree_grid.width - 1
			{
				first_pass[i].0 = true;
			} else {
				let [this, right, below] = first_pass
					.get_many_mut([i, i + 1, i + tree_grid.width])
					.unwrap();

				let height = this.1;

				// Note the |=, which will preserve visibility from the initial above,left pass
				this.0 |= height > below.1 || height > right.2;
				this.1 = height.max(below.1);
				this.2 = height.max(right.2);
			}
		});

		// Count the number of visible trees
		first_pass.iter().filter(|(vis, _, _)| *vis).count()
	}
}

mod part2 {
	use super::TreeGrid;

	#[derive(Clone, Copy)]
	/// Convenience struct for keeping track of how far can be seen in a direction from a tree,
	/// and the height of the tree blocking us from seeing further
	struct ViewDistance {
		/// How far can be seen in a direction
		distance: usize,
		/// The height of the tree that is blocking sight in a direction.
		/// Or None if we can see all the way to an edge
		height: Option<u8>,
	}

	impl ViewDistance {
		/// Construct a `ViewDistance` for a tree which is on the edge - where we can't see any trees,
		/// and there is no tree blocking our sight.
		fn edge() -> Self {
			Self {
				distance: 0,
				height: None,
			}
		}
	}

	/// A convenience struct for keeping track of how far we can see from a tree in every direction.
	struct ViewDirections {
		above: Option<ViewDistance>,
		left: Option<ViewDistance>,
		right: Option<ViewDistance>,
		below: Option<ViewDistance>,
	}

	#[derive(Clone, Copy)]
	/// A convenience struct for encoding which direction we want to look.
	/// Above and Below must know about how wide each row in the tree grid is.
	enum Direction {
		Above(usize),
		Left,
		Right,
		Below(usize),
	}

	impl Direction {
		/// Offset an index (`idx`) into another index in a certain direction some number of steps (`mult`).
		fn offset(&self, idx: usize, mult: usize) -> usize {
			match self {
				Direction::Above(width) => idx - mult * width,
				Direction::Left => idx - mult,
				Direction::Right => idx + mult,
				Direction::Below(width) => idx + mult * width,
			}
		}
	}

	impl ViewDirections {
		/// Return the `ViewDistance` associated with a particular direction
		fn in_dir(&self, direction: Direction) -> Option<ViewDistance> {
			match direction {
				Direction::Above(_) => self.above,
				Direction::Left => self.left,
				Direction::Right => self.right,
				Direction::Below(_) => self.below,
			}
		}
	}

	/// Find the `ViewDistance` from a tree in a particular direction
	fn find_view_distance(
		views: &[ViewDirections],
		tree_grid: &TreeGrid,
		idx: usize,
		direction: Direction,
	) -> ViewDistance {
		// The height of the tree we're finding the view distance for
		let height = tree_grid.heights[idx];
		// The height of the tree we're currently looking at (if it exists). To start, the neighboring tree
		// in the direction we're looking.
		let mut maybe_view_height = tree_grid.heights.get(direction.offset(idx, 1)).copied();

		// The current distance we can look. If our neighbor exists, then we can see it, and the starting distance is 1.
		// Otherwise, we have no neighbor and the starting distance is 0, since there is no tree to see.
		let mut distance = usize::from(maybe_view_height.is_some());

		// Continue looking at trees past the one we're looking at as long as there is a tree to look at, and its height is less than our height
		while let Some(view_height) = maybe_view_height && view_height < height {
			// Look at the pre-computed information of the tree we're looking at
			let view = views[direction.offset(idx, distance)]
				.in_dir(direction)
				.unwrap();

			// We know we can look past this tree. And if so, we know we can look past it at least as far as we would be able to see
			// from that tree.
			distance += view.distance;

			// Then we just need to check if we can look past the tree that would block the tree we're looking a
			maybe_view_height = view.height;
		}

		ViewDistance {
			distance,
			height: maybe_view_height,
		}
	}

	pub(super) fn highest_scenic_score(tree_grid: &TreeGrid) -> usize {
		// Default initialise the views vector, which keep track of our partial results
		// for calculating full results and also calculating partial results of other trees
		let mut views = tree_grid
			.heights
			.iter()
			.map(|_| ViewDirections {
				above: None,
				left: None,
				right: None,
				below: None,
			})
			.collect::<Vec<_>>();

		// Similar to part 1, do a first partial pass that only calculates partial results in the above,left directions.
		// Since we are iterating forward, the partial results for all of the trees in each tree's above,left directions
		// have already been calculated, so we can use those.
		(0..views.len()).for_each(|i| {
			// Top left corner
			if i == 0 {
				views[i].above = Some(ViewDistance::edge());
				views[i].left = Some(ViewDistance::edge());
			}
			// Top edge
			else if i < tree_grid.width {
				views[i].above = Some(ViewDistance::edge());
				views[i].left = Some(find_view_distance(&views, tree_grid, i, Direction::Left));
			}
			// Left edge
			else if i % tree_grid.width == 0 {
				views[i].above = Some(find_view_distance(
					&views,
					tree_grid,
					i,
					Direction::Above(tree_grid.width),
				));
				views[i].left = Some(ViewDistance::edge());
			}
			// Inside
			else {
				views[i].above = Some(find_view_distance(
					&views,
					tree_grid,
					i,
					Direction::Above(tree_grid.width),
				));
				views[i].left = Some(find_view_distance(&views, tree_grid, i, Direction::Left));
			}
		});

		// Similar to part 1, now calculate partial results for right,below directions. Reverse iteration
		// to keep property allowing us to use other partial results.
		(0..views.len()).rev().for_each(|i| {
			// Bottom right corner
			if i == views.len() {
				views[i].right = Some(ViewDistance::edge());
				views[i].below = Some(ViewDistance::edge());
			}
			// Bottom edge
			else if i + tree_grid.width > views.len() {
				views[i].right = Some(find_view_distance(&views, tree_grid, i, Direction::Right));
				views[i].below = Some(ViewDistance::edge());
			}
			// Right edge
			else if i % tree_grid.width == tree_grid.width - 1 {
				views[i].right = Some(ViewDistance::edge());
				views[i].below = Some(find_view_distance(
					&views,
					tree_grid,
					i,
					Direction::Below(tree_grid.width),
				));
			}
			// Inside
			else {
				views[i].right = Some(find_view_distance(&views, tree_grid, i, Direction::Right));
				views[i].below = Some(find_view_distance(
					&views,
					tree_grid,
					i,
					Direction::Below(tree_grid.width),
				));
			}
		});

		views
			.iter()
			.map(|v| {
				// Scenic scores are product of distances in each direction (partial results)
				v.above.unwrap().distance
					* v.below.unwrap().distance
					* v.left.unwrap().distance
					* v.right.unwrap().distance
			})
			// Find max scenic score
			.max()
			.unwrap()
	}
}
fn main() -> Result<()> {
	let args = Args::parse();

	let mut file = File::open(args.input_file)?;

	let mut input = String::new();
	file.read_to_string(&mut input)?;

	let tree_grid = input.parse()?;

	match args.mode {
		Mode::NumVisible => println!("{}", part1::visible_trees(&tree_grid)),
		Mode::ScenicScore => println!("{}", part2::highest_scenic_score(&tree_grid)),
	}

	Ok(())
}

#[cfg(test)]
mod tests {
	use super::*;

	// The example prompt
	static PROMPT: &str = "30373
25512
65332
33549
35390";

	#[test]
	fn example() {
		let tree_grid = PROMPT.parse::<TreeGrid>().unwrap();
		assert_eq!(part1::visible_trees(&tree_grid), 21);
		assert_eq!(part2::highest_scenic_score(&tree_grid), 8);
	}
}
