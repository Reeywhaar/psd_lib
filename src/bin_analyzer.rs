//! psd_analyzer binary
//!
//! Shows analyze information for psd file
//!
//! ```
//! usage: $: psd_analyzer [--fullpath] [--flat] [--with-size] [--with-hash] file.psd [> analysis.txt]
//! 	--fullpath: show full path
//! 	--flat: don't indent blocks
//! 	--with-size: show block size in bytes
//! 	--with-hash: append hash to each block
//! ```

extern crate bin_diff;
extern crate psd_lib;
extern crate sha2;

use bin_diff::indexes::WithIndexes;
use psd_lib::psd_file::PSDFile;
use sha2::{Digest, Sha256};
use std::cmp::max;
use std::env;
use std::fs::File;
use std::io::{stdout, BufWriter, Read, Seek, SeekFrom, Write};
use std::process::exit;

fn compute_hash<T: Read>(input: &mut T) -> String {
	let mut hasher = Sha256::default();

	let mut buf: Vec<u8> = vec![0; 1024 * 64];
	while let Ok(x) = input.read(&mut buf) {
		if x == 0 {
			break;
		}
		let slice = &buf[0..x];
		hasher.input(slice);
	}

	return hasher
		.result()
		.iter()
		.map(|b| format!("{:02x}", b))
		.collect::<Vec<String>>()
		.join("");
}

fn main() {
	let args = env::args().skip(1);

	let mut path: Option<String> = None;
	let mut fullpath = false;
	let mut flat = false;
	let mut with_size = false;
	let mut with_hash = false;

	for arg in args {
		match arg.as_ref() {
			"--fullpath" => fullpath = true,
			"--flat" => flat = true,
			"--with-size" => with_size = true,
			"--with-hash" => with_hash = true,
			x => {
				path = Some(x.to_string());
			}
		}
	}

	let path = path.unwrap_or_else(|| {
		eprintln!("Input file is not provided");
		exit(1);
	});

	let file = File::open(path).unwrap_or_else(|_| {
		eprintln!("Error reading input psd");
		exit(1);
	});
	let mut file_h = file.try_clone().unwrap();
	let mut file = PSDFile::new(file);

	let output = stdout();
	let mut output = output.lock();
	let mut output = BufWriter::with_capacity(1024 * 64, &mut output);

	let indexes = file.get_indexes().unwrap_or_else(|_| {
		eprintln!("Cannot get indexes");
		exit(1);
	});

	for (item, start, size) in indexes {
		let indent: usize = match flat {
			true => 0,
			false => {
				let indent = item.matches("/").count();
				let indentb = item.matches(":").count();
				indent + indentb
			}
		};
		let mut s = item.clone();
		if !fullpath {
			let index = max(
				s.clone().rfind("/").unwrap_or(0),
				s.clone().rfind(":").unwrap_or(0),
			);
			if index != 0 {
				s = s.split_at(index + 1).1.to_string();
			};
		};
		let mut end_s = (start + size).to_string();
		if with_size {
			end_s = format!("{} ({})", end_s, size);
		};
		let mut out = format!("{}{} : {} {}", "  ".repeat(indent), s, start, end_s);
		if with_hash {
			let max_size = 1024 * 1024 * 100;
			if size != 0 && size < max_size {
				let hash = {
					&file_h.seek(SeekFrom::Start(start.clone()));
					let mut file_p = (&file_h).take(size.clone());
					let hash = compute_hash(&mut file_p);
					hash
				};
				out = format!("{}   {}", hash, out);
			} else if size > max_size {
				out = format!("{:-<64}   {}", "", out);
			} else {
				out = format!("{:.<64}   {}", "", out);
			}
		};
		out = format!("{}\n", out);
		let res = output.write(out.as_bytes());
		if res.is_err() {
			eprintln!("Error while reading file");
			exit(1);
		}
	}

	let res = output.flush();
	if res.is_err() {
		eprintln!("Error while flushing final data");
		exit(1);
	}
}
