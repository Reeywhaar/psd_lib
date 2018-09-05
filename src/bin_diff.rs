//! psd_diff binary
//!
//! Creates or applies diff of psd file
//!
//! ```
//! usage: $: psd_diff create|apply [...args]
//! usage: $: psd_diff create file_a.psd file_b.psd file_a_b.psd.diff
//! usage: $: psd_diff apply file_a.psd file_a_b.psd.diff file_b.psd
//! ```

extern crate psd_lib;

use psd_lib::diff::{apply_diff as i_apply_diff, create_diff as i_create_diff};
use psd_lib::psd_file::PSDFile;
use std::env::args;
use std::fs::File;
use std::process::exit;

fn create_diff(old: &str, new: &str, output_path: &str) -> Result<(), &'static str> {
	let mut old = PSDFile::new(File::open(old).or(Err("Cannot open original file"))?);
	let mut new = PSDFile::new(File::open(new).or(Err("Cannot open edited file"))?);
	let mut output = File::create(output_path).or(Err("Cannot open output file"))?;

	i_create_diff(&mut old, &mut new, &mut output).or(Err("Cannot create diff"))?;

	Ok(())
}

fn apply_diff(old_path: &str, diff_path: &str, output_path: &str) -> Result<(), &'static str> {
	let mut file = File::open(old_path).or(Err("Cannot open original file"))?;
	let mut diff = File::open(diff_path).or(Err("Cannot open diff file"))?;
	let mut stdo = File::create(output_path).or(Err("Cannot open output file"))?;

	i_apply_diff(&mut file, &mut diff, &mut stdo).or(Err("Error applying diff"))?;

	Ok(())
}

fn process() -> Result<(), &'static str> {
	let args: Vec<String> = args().skip(1).collect();
	let usage_str = "usage: $action create|apply [...args]";
	if args.len() == 0 {
		return Err(usage_str);
	}
	let action = args[0].clone();
	match action.as_ref() {
		"create" => {
			if args.len() < 4 {
				return Err(
					"usage: bin_diff create $original_path $edited_path $original_to_edited_diff_path",
				);
			};
			return create_diff(&args[1], &args[2], &args[3]);
		}
		"apply" => {
			if args.len() < 4 {
				return Err("usage: bin_diff apply $original_path $diff_file $edited_file");
			};
			return apply_diff(&args[1], &args[2], &args[3]);
		}
		_ => return Err(usage_str),
	};
}

fn main() {
	let res = process();
	if res.is_err() {
		eprintln!("{}", res.unwrap_err());
		exit(1);
	};
}
