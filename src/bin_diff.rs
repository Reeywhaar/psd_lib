//! psd_diff binary
//!
//! Creates or applies diff of psd file
//!
//! ```
//! usage:
//! $: psd_diff create|apply|combine [...args]
//!
//! $: psd_diff create file_a.psd file_b.psd file_a_b.psd.diff
//!     output file can be substituted with "-", what means output to stdout
//!
//! $: psd_diff apply file_a.psd [...file_a_b.psd.diff>1] file_b.psd
//!     output file can be substituted with "-", what means output to stdout
//!
//! $: psd_diff combine [...a.psd.diff>2] output.psd.diff
//!     output file can be substituted with "-", what means output to stdout
//! ```

extern crate psd_lib;

use psd_lib::diff::{
	apply_diff as apply, apply_diffs_vec as applyd, combine_diffs_vec as combine,
	create_diff as create,
};
use psd_lib::psd_file::PSDFile;
use std::env::args;
use std::fs::{remove_file, rename, File};
use std::io::{stdout, Write};
use std::process::exit;
use std::time::{SystemTime, UNIX_EPOCH};

fn timestamp() -> String {
	let now = SystemTime::now();
	let elapsed = now
		.duration_since(UNIX_EPOCH)
		.expect("Error getting timestamp");
	return elapsed.as_secs().to_string();
}

fn create_tempname(path: &str) -> String {
	return format!("{}.tmp.{}", path, timestamp());
}

fn create_diff(old: &str, new: &str, output_path: &str) -> Result<(), String> {
	let mut old = PSDFile::new(File::open(old).or(Err("Cannot open original file".to_string()))?);
	let mut new = PSDFile::new(File::open(new).or(Err("Cannot open edited file".to_string()))?);

	let tempname = if output_path != "-" {
		create_tempname(output_path)
	} else {
		"-".to_string()
	};

	let mut output: Box<dyn Write> = if output_path == "-" {
		Box::new(stdout())
	} else {
		Box::new(File::create(&tempname).or(Err("Cannot open output file".to_string()))?)
	};

	let res = create(&mut old, &mut new, &mut output).or(Err("Cannot create diff".to_string()));

	if res.is_err() {
		if output_path != "-" {
			remove_file(&tempname).map_err(|err| err.to_string())?;
		}
		return res;
	} else {
		if output_path != "-" {
			rename(&tempname, output_path).map_err(|err| err.to_string())?;
		}
	}

	Ok(())
}

fn apply_diff(old_path: &str, diff_path: &str, output_path: &str) -> Result<(), String> {
	let mut file = File::open(old_path).or(Err("Cannot open original file".to_string()))?;
	let mut diff = File::open(diff_path).or(Err("Cannot open diff file".to_string()))?;

	let tempname = if output_path != "-" {
		create_tempname(output_path)
	} else {
		"-".to_string()
	};

	let mut output: Box<dyn Write> = if output_path == "-" {
		Box::new(stdout())
	} else {
		Box::new(File::create(&tempname).or(Err("Cannot open output file".to_string()))?)
	};

	let res = apply(&mut file, &mut diff, &mut output).or(Err("Error applying diff".to_string()));

	if res.is_err() {
		if output_path != "-" {
			remove_file(&tempname).map_err(|err| err.to_string())?;
		}
		return res;
	} else {
		if output_path != "-" {
			rename(&tempname, output_path).map_err(|err| err.to_string())?;
		}
	}

	Ok(())
}

fn apply_diff_vec(old_path: &str, diff_paths: &[&str], output_path: &str) -> Result<(), String> {
	let mut file = File::open(old_path).or(Err("Cannot open original file".to_string()))?;
	let mut diffs = vec![];
	for path in diff_paths {
		let diff = File::open(path).or(Err("Cannot open diff file".to_string()))?;
		diffs.push(diff);
	}

	let tempname = if output_path != "-" {
		create_tempname(output_path)
	} else {
		"-".to_string()
	};

	let mut output: Box<dyn Write> = if output_path == "-" {
		Box::new(stdout())
	} else {
		Box::new(File::create(&tempname).or(Err("Cannot open output file".to_string()))?)
	};

	let res = applyd(&mut file, &mut diffs, &mut output).or(Err("Error applying diff".to_string()));

	if res.is_err() {
		if output_path != "-" {
			remove_file(&tempname).map_err(|err| err.to_string())?;
		}
		return res;
	} else {
		if output_path != "-" {
			rename(&tempname, output_path).map_err(|err| err.to_string())?;
		}
	}

	Ok(())
}

fn combine_diffs(paths: &[&str], output_path: &str) -> Result<(), String> {
	let mut files = vec![];
	for path in paths {
		let file = File::open(path).or(Err(format!("Cannot open path: {}", path)))?;
		files.push(file);
	}

	let tempname = if output_path != "-" {
		create_tempname(output_path)
	} else {
		"-".to_string()
	};

	let mut output: Box<dyn Write> = if output_path == "-" {
		Box::new(stdout())
	} else {
		Box::new(File::create(&tempname).or(Err("Cannot open output file".to_string()))?)
	};

	let res = combine(&mut files, &mut output).map_err(|err| format!("{}", err));

	if res.is_err() {
		if output_path != "-" {
			remove_file(&tempname).map_err(|err| err.to_string())?;
		}
		return res;
	} else {
		if output_path != "-" {
			rename(&tempname, output_path).map_err(|err| err.to_string())?;
		}
	}

	Ok(())
}

fn process() -> Result<(), String> {
	let args: Vec<String> = args().skip(1).collect();
	let usage_str = "usage: $action create|apply [...args]".to_string();
	if args.len() == 0 {
		return Err(usage_str);
	}
	let action = args[0].clone();
	match action.as_ref() {
		"create" => {
			if args.len() < 4 {
				return Err(
					"usage: bin_diff create $original_path $edited_path $original_to_edited_diff_path".to_string(),
				);
			};
			return create_diff(&args[1], &args[2], &args[3]);
		}
		"apply" => {
			if args.len() < 4 {
				return Err(
					"usage: bin_diff apply $original_path [...$diff_file>=1] $edited_file"
						.to_string(),
				);
			};
			if args.len() == 4 {
				return apply_diff(&args[1], &args[2], &args[3]);
			}

			let file = args[1].clone();
			let diffs = (&args[2..(args.len() - 1)]).clone();
			let output = args[args.len() - 1].clone();

			eprintln!("{:?} {:?} {:?}c", file, diffs, output);

			return apply_diff_vec(
				&file,
				&diffs.iter().map(|x| x.as_ref()).collect::<Vec<_>>(),
				&output,
			);
		}
		"combine" => {
			if args.len() < 4 {
				return Err("usage: bin_diff [...$diff_file>2] $output".to_string());
			};
			let args = &args[1..].to_vec();
			let (mut diffs, output) = args.split_at(args.len() - 1);
			return combine_diffs(
				&diffs.iter().map(|x| x.as_ref()).collect::<Vec<_>>(),
				&output[0],
			);
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
