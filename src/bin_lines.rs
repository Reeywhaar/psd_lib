extern crate psd_lib;
extern crate sha2;

use psd_lib::diff::LinesWithHashIterator;
use std::env::args;
use std::fs::File;
use std::io::{stdout, BufWriter, Write};
use std::process::exit;

fn pad_right(string: &str, len: usize) -> String {
	if string.len() >= len {
		return string.to_string();
	}
	let diff = len - string.len();
	return format!("{}{}", string, " ".repeat(diff));
}

fn main() {
	let args = args().skip(1);
	if args.len() == 0 {
		eprintln!("usage: bin_lines [--truncate] [...file_path > 0]");
		exit(1);
	};
	let mut truncate = false;
	let mut paths: Vec<String> = vec![];
	for arg in args {
		match arg.as_ref() {
			"--truncate" => truncate = true,
			x => paths.push(x.to_string()),
		};
	}
	let padding_length = {
		if truncate {
			70
		} else {
			100
		}
	};

	let stdout = stdout();
	let stdout = stdout.lock();
	let mut stdout = BufWriter::with_capacity(1024 * 64, stdout);
	let mut data = {
		let mut o: Vec<LinesWithHashIterator<File>> = vec![];
		for path in &paths {
			let file = File::open(&path).unwrap();
			let it = LinesWithHashIterator::new(file).unwrap();
			o.push(it);
		}
		o
	};

	{
		let mut header = "".to_string();
		for path in &paths {
			let path = format!("{}", path);
			header = format!("{} {}|", header, pad_right(&path, padding_length));
		}
		header = format!("{}\n", header);
		stdout.write(header.as_bytes()).unwrap();
	}

	loop {
		let mut items: Vec<_> = (&mut data).into_iter().map(|x| x.next()).collect();
		if (&mut items).into_iter().all(|x| x.is_none()) {
			break;
		}
		let mut line = "".to_string();

		for item in &items {
			if item.is_none() {
				line = format!("{} {}|", line, pad_right("", padding_length));
				continue;
			}
			let (label, start, size, hash) = item.clone().unwrap();

			let mut label = label.clone();
			if truncate {
				if label.len() > 30 {
					label = format!("{}...{}", &label[0..10], &label[(label.len() - 17)..]);
				}
			};

			let mut hash = hash.clone();
			hash.truncate(16);

			let o = format!("{} {} : {} {}", hash, label, start, size);
			line = format!("{} {}|", line, pad_right(&o, padding_length));
		}

		line = format!("{}\n", line);
		stdout.write(line.as_bytes()).unwrap();
	}

	let res = stdout.flush();
	if res.is_err() {
		eprintln!("Error while writing");
		exit(1);
	};
}
