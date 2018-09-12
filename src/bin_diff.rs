//! psd_diff binary
//!
//! Creates or applies diff of psd file
//!
//! ```
//! usage:
//! $: psd_diff measure|create|apply|combine [...args]
//!
//! $: psd_diff measure [--in-bytes] file_a.psd file_b.psd
//!     --in-bytes: output size in bytes instead of human readable format
//!
//! $: psd_diff create file_a.psd file_b.psd file_a_b.psd.diff
//!     output file can be substituted with "-", what means output to stdout
//!
//! $: psd_diff apply file_a.psd [...file_a_b.psd.diff>1] file_b.psd
//!     output file can be substituted with "-", what means output to stdout
//!
//! $: psd_diff combine [...a.psd.diff>2] output.psd.diff
//!     output file can be substituted with "-", what means output to stdout
//!
//! Also setting environment PSDDIFF_VERBOSE=true will make command print elapsed time
//! ```

extern crate bin_diff;
extern crate psd_lib;
mod proxy_file;

use proxy_file::ProxyFile;
use psd_lib::diff::{
	apply_diff as apply, apply_diffs_vec as applyd, combine_diffs_vec as combine,
	create_diff as create, measure_diff_size as measure,
};
use psd_lib::psd_file::PSDFile;
use std::env::{args, var};
use std::fs::File;
use std::process::exit;
use std::sync::mpsc::channel;
use std::thread;
use std::time::Duration;

fn printdots() -> Box<Fn() -> ()> {
	let (ttx, rrx) = channel();
	let (tx, rx) = channel();
	thread::spawn(move || {
		let mut counter = 0u8;
		let mut elapsed = 0;
		loop {
			if let Ok(_) = rrx.try_recv() {
				eprint!("\n");
				break;
			} else {
				thread::sleep(Duration::from_millis(10));
				if counter == 100 {
					elapsed += 1;
					eprint!("\relapsed: {}s", elapsed);
					counter = 0;
				} else {
					counter += 1;
				}
			}
		}
		tx.send(()).unwrap();
	});

	let out = move || {
		ttx.send(()).unwrap();
		rx.recv().unwrap();
	};

	return Box::new(out);
}

const GIGABYTE: f64 = 1_063_256_064.0;
const MEGABYTE: f64 = 1_048_576.0;
const KILOBYTE: f64 = 1_024.0;

fn measure_diff(old: &str, new: &str, human_readable: bool) -> Result<(), String> {
	let mut old = PSDFile::new(File::open(old).or(Err("Cannot open original file".to_string()))?);
	let mut new = PSDFile::new(File::open(new).or(Err("Cannot open edited file".to_string()))?);

	let printdots = match var("PSDDIFF_VERBOSE") {
		Ok(ref x) if x == "true" => Some(printdots()),
		_ => None,
	};
	let size = measure(&mut old, &mut new).or(Err("Error while measuring diff".to_string()))?;
	if let Some(stopdots) = printdots {
		stopdots();
	};
	if human_readable {
		let gb = (size as f64 / GIGABYTE).floor();
		let mb = ((size as f64 - (gb * GIGABYTE)) / MEGABYTE).floor();
		let kb = ((size as f64 - (gb * GIGABYTE) - (mb * MEGABYTE)) / KILOBYTE).floor();
		let b = size as f64 - (gb * GIGABYTE) - (mb * MEGABYTE) - (kb * KILOBYTE);
		println!("{}GB {}MB {}KB {}B", gb, mb, kb, b);
	} else {
		println!("{}", size);
	}

	Ok(())
}

fn create_diff(old: &str, new: &str, output_path: &str) -> Result<(), String> {
	let mut old = PSDFile::new(File::open(old).or(Err("Cannot open original file".to_string()))?);
	let mut new = PSDFile::new(File::open(new).or(Err("Cannot open edited file".to_string()))?);
	let mut output = ProxyFile::from(output_path.to_string());

	let printdots = match var("PSDDIFF_VERBOSE") {
		Ok(ref x) if x == "true" => Some(printdots()),
		_ => None,
	};
	let res = create(&mut old, &mut new, &mut output);
	if let Some(stopdots) = printdots {
		stopdots();
	}

	if let Err(e) = res {
		output.set_err(e);
		return Err("Cannot create diff".to_string());
	}

	Ok(())
}

fn apply_diff(old_path: &str, diff_path: &str, output_path: &str) -> Result<(), String> {
	let mut file = File::open(old_path).or(Err("Cannot open original file".to_string()))?;
	let mut diff = File::open(diff_path).or(Err("Cannot open diff file".to_string()))?;
	let mut output = ProxyFile::from(output_path.to_string());

	let printdots = match var("PSDDIFF_VERBOSE") {
		Ok(ref x) if x == "true" => Some(printdots()),
		_ => None,
	};
	let res = apply(&mut file, &mut diff, &mut output);
	if let Some(stopdots) = printdots {
		stopdots();
	}

	if let Err(e) = res {
		output.set_err(e);
		return Err("Error applying diff".to_string());
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
	let mut output = ProxyFile::from(output_path.to_string());

	let printdots = match var("PSDDIFF_VERBOSE") {
		Ok(ref x) if x == "true" => Some(printdots()),
		_ => None,
	};
	let res = applyd(&mut file, &mut diffs, &mut output);
	if let Some(stopdots) = printdots {
		stopdots();
	}

	if let Err(e) = res {
		output.set_err(e);
		return Err("Error applying diff".to_string());
	}

	Ok(())
}

fn combine_diffs(paths: &[&str], output_path: &str) -> Result<(), String> {
	let mut files = vec![];
	for path in paths {
		let file = File::open(path).or(Err(format!("Cannot open path: {}", path)))?;
		files.push(file);
	}
	let mut output = ProxyFile::from(output_path.to_string());

	let printdots = match var("PSDDIFF_VERBOSE") {
		Ok(ref x) if x == "true" => Some(printdots()),
		_ => None,
	};
	let res = combine(&mut files, &mut output);
	if let Some(stopdots) = printdots {
		stopdots();
	}

	if let Err(e) = res {
		let outerr = format!("{}", e);
		output.set_err(e);
		return Err(outerr);
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
		"measure" => {
			let usage_str =
				"usage: bin_diff measure [--in-bytes] $original_path $edited_path".to_string();
			if args.len() < 3 {
				return Err(usage_str);
			};
			let mut human_readable = true;
			let mut original = None;
			let mut edited = None;
			for arg in args.iter().skip(1) {
				match arg.as_ref() {
					"--in-bytes" => {
						human_readable = false;
					}
					val => {
						if original.is_none() {
							original = Some(val.to_string());
							continue;
						};
						if edited.is_none() {
							edited = Some(val.to_string());
							continue;
						};
					}
				}
			}

			if original.is_none() || edited.is_none() {
				return Err(usage_str);
			}
			return measure_diff(&original.unwrap(), &edited.unwrap(), human_readable);
		}
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
