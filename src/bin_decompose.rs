//! psd_decompose binary
//!
//! `psd_decompose` allows to decompose psd file into chunks of objects which it store in the `decomposed_objects directory` and `file.psd.decomposed` text file next to original file.
//! The reason for this binary is an ability to decompose multiple files in the same directory and store them as chunks, therefore reducing the total size because of shared chunks
//!
//! ### Usage
//!
//! ```bash
//! $: psd_decompose [...file.psd > 1]
//!
//! $: psd_decompose --restore [--prefix=string] [--postfix=string] [...file.psd.decomposed > 1]
//!    --prefix:  prepend string to restored filename
//!    --postfix: append string to restored filename before extension
//!
//! $: psd_decompose --sha [...file > 1]
//!    compute sha256 hash of given prospective restored files or ordinary files. Usefull to check that restore will be correct.
//!
//! $: psd_decompose --remove [...file.decomposed > 1]
//!    removes decomposed index file and rebuild (actually gather all the hashes from other files in the directory and removes hashes which are orphaned) decomposed_opjects directory.
//!
//! $: psd_decompose --cleanup
//!    perform cleanup of "decomposed_objects" directory which consists of populating unique index of every hash of every .decomposed file and removing every hash which doesn't said index contains.
//! ```
//!

extern crate bin_diff;
extern crate psd_lib;
extern crate sha2;

mod once_option;

use bin_diff::lines_with_hash_iterator::LinesWithHashIterator;
use once_option::OnceOption;
use psd_lib::psd_file::PSDFile;
use sha2::{Digest, Sha256};
use std::env::args;
use std::fs::{create_dir_all, read_dir, remove_file, File};
use std::io::{copy, BufRead, BufReader, BufWriter, Read, Seek, SeekFrom, Write};
use std::path::PathBuf;
use std::process::exit;

const EMPTY_HASH: &str = "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855";

fn get_objects<'a, T: 'a + Read + Seek>(
	input: T,
) -> Result<LinesWithHashIterator<PSDFile<T>>, String> {
	let file = PSDFile::new(input);
	return LinesWithHashIterator::new(file);
}

fn decompose_file(paths: &Vec<PathBuf>) -> Result<(), String> {
	for path in paths {
		let mut input = File::open(&path).or(Err(format!("Cannot open {:?}", path)))?;
		let objects = {
			let obj = get_objects(&mut input)?;
			obj.collect::<Vec<_>>()
		};

		eprintln!("processing {:?}", path);

		let objdir = path.parent().expect("Parent directory not found");
		let mut objdir = PathBuf::from(objdir);
		objdir.push("decomposed_objects");

		for (_label, start, size, hash) in &objects {
			if hash == EMPTY_HASH {
				continue;
			};

			let mut hashdir = objdir.clone();
			hashdir.push(&hash[0..2]);
			let mut hashloc = hashdir.clone();
			hashloc.push(hash);

			if hashloc.exists() {
				continue;
			}

			if !hashdir.exists() {
				create_dir_all(&hashdir)
					.or(Err(format!("Cannot create hash directory: {:?}", hashdir)))?;
			}

			input
				.seek(SeekFrom::Start(*start))
				.or(Err(format!("Cannot seek {:?}", path)))?;
			let mut chunk = Read::by_ref(&mut input).take(*size);
			let mut hashfile = File::create(&hashloc)
				.or(Err(format!("Cannot create hash object: {:?}", hashloc)))?;
			eprintln!("writing {:?}", hashloc);
			copy(&mut chunk, &mut hashfile)
				.or(Err(format!("Cannot write hash object: {:?}", hashloc)))?;
		}

		let mut indexfileloc = PathBuf::from(path);
		if indexfileloc.extension().is_some() {
			let ext = (&indexfileloc)
				.extension()
				.map(|x| x.to_os_string().into_string().unwrap())
				.unwrap();
			indexfileloc.set_extension(format!("{}.{}", ext, "decomposed"));
		} else {
			indexfileloc.set_extension("decomposed");
		}

		let indexfile = File::create(&indexfileloc)
			.or(Err(format!("Cannot create index file: {:?}", indexfileloc)))?;
		let mut indexfile = BufWriter::with_capacity(1024, indexfile);

		for (_, _, _, hash) in objects {
			indexfile
				.write(format!("{}\n", hash).as_bytes())
				.or(Err(format!(
					"Cannot write to index file: {:?}",
					indexfileloc
				)))?;
		}

		indexfile.flush().or(Err("Cannot flush index file"))?;
	}

	Ok(())
}

fn restore_file(paths: &Vec<PathBuf>, prefix: &str, postfix: &str) -> Result<(), String> {
	for path in paths {
		if path.extension().unwrap() != "decomposed" {
			return Err("File extension should be \".decomposed\" or program fails".to_string());
		}

		let file = File::open(&path).or(Err(format!("Cannot open path: {:?}", path)))?;
		let file = BufReader::with_capacity(1024, file);

		let mut restored_loc = path.to_path_buf();
		restored_loc.set_extension("");
		if prefix != "" || postfix != "" {
			let stem = restored_loc
				.file_stem()
				.map(|x| x.to_os_string().into_string().unwrap_or("".to_string()))
				.unwrap_or("".to_string());
			let ext = restored_loc
				.extension()
				.map(|x| x.to_os_string().into_string().unwrap_or("".to_string()))
				.map(|x| format!(".{}", x))
				.unwrap_or("".to_string());
			restored_loc.set_file_name(format!("{}{}{}{}", prefix, stem, postfix, ext));
		}

		let objdir = path.parent().expect("Parent directory not found");
		let mut objdir = PathBuf::from(objdir);
		objdir.push("decomposed_objects");

		let mut output_file = BufWriter::with_capacity(
			1024 * 64,
			File::create(&restored_loc).or(Err(format!(
				"Cannot create file to restore: {:?}",
				restored_loc
			)))?,
		);

		for hash in file.lines() {
			let hash = hash.map_err(|e| e.to_string())?;

			if hash == EMPTY_HASH {
				continue;
			}

			let mut hashdir = objdir.clone();
			hashdir.push(&hash[0..2]);
			let mut hashloc = hashdir.clone();
			hashloc.push(hash);

			if !hashloc.exists() {
				return Err(format!("hash {:?} doesn't exists", hashloc));
			}

			let mut hashfile =
				File::open(&hashloc).or(Err(format!("Cannot open hash {:?}", &hashloc)))?;
			copy(&mut hashfile, &mut output_file).map_err(|e| e.to_string())?;
		}

		output_file.flush().or(Err(format!(
			"Cannot flush to output file: {:?}",
			restored_loc
		)))?;
	}

	Ok(())
}

fn output_shasum(paths: &Vec<PathBuf>) -> Result<(), String> {
	for path in paths {
		let file = File::open(&path).or(Err(format!("Cannot open path: {:?}", path)))?;
		if path.extension().is_some() && path.extension().unwrap() == "decomposed" {
			let file = BufReader::with_capacity(1024, file);
			let objdir = path.parent().expect("Parent directory not found");
			let mut objdir = PathBuf::from(objdir);
			objdir.push("decomposed_objects");

			let mut hasher = Sha256::default();
			let mut buf = vec![0; 1024 * 64];

			for hash in file.lines() {
				let hash = hash.map_err(|e| e.to_string())?;

				if hash == EMPTY_HASH {
					continue;
				}

				let mut hashdir = objdir.clone();
				hashdir.push(&hash[0..2]);
				let mut hashloc = hashdir.clone();
				hashloc.push(hash);

				if !hashloc.exists() {
					return Err(format!("hash {:?} doesn't exists", hashloc));
				}

				let mut hashfile =
					File::open(&hashloc).or(Err(format!("Cannot open hash {:?}", &hashloc)))?;

				loop {
					let read = hashfile
						.read(&mut buf)
						.or(Err(format!("Cannot read decomposed chunk {:?}", &hashloc)))?;
					if read == 0 {
						break;
					};
					hasher.input(&buf[..read]);
				}
			}

			let hash = hasher
				.result()
				.iter()
				.map(|b| format!("{:02x}", b))
				.collect::<Vec<String>>()
				.join("");

			println!("{} - {}", hash, path.to_string_lossy());
		} else {
			let mut file = BufReader::with_capacity(1024 * 64, file);

			let mut hasher = Sha256::default();
			let mut buf = vec![0; 1024 * 64];

			loop {
				let read = file
					.read(&mut buf)
					.or(Err(format!("Cannot read file {:?}", &path)))?;
				if read == 0 {
					break;
				};
				hasher.input(&buf[..read]);
			}

			let hash = hasher
				.result()
				.iter()
				.map(|b| format!("{:02x}", b))
				.collect::<Vec<String>>()
				.join("");

			println!("{} - {}", hash, path.to_string_lossy());
		}
	}

	return Ok(());
}

fn cleanup() -> Result<(), String> {
	let mut objdir = PathBuf::from(".");
	objdir.push("decomposed_objects");
	if !objdir.exists() {
		return Err("decomposed_objects directory doesn't exists".to_string());
	};

	if !objdir.is_dir() {
		return Err("decomposed_objects is not directory".to_string());
	}

	let indexes = read_dir(&objdir)
		.or(Err("Cannot read decomposed_objects directory".to_string()))?
		.scan((), |_, x| x.ok())
		.flat_map(|sub_dir| {
			return read_dir(sub_dir.path()).unwrap();
		})
		.scan((), |_, x| x.ok())
		.map(|x| x.path());

	let rindexes = read_dir(".")
		.or(Err("Cannot read directory".to_string()))?
		.scan((), |_, x| x.ok())
		.filter_map(|file| {
			let path = file.path();
			return path.extension().and_then(|ext| {
				if ext == "decomposed" {
					return Some(path.clone());
				} else {
					return None;
				}
			});
		})
		.flat_map(|file| BufReader::new(File::open(file).unwrap()).lines())
		.scan((), |_, x| x.ok())
		.map(|x| {
			let mut o = objdir.clone();
			o.push(&x[..2]);
			o.push(&x);
			o
		})
		.fold(vec![], |mut c, hash| {
			if !c.contains(&hash) {
				c.push(hash);
			};
			return c;
		});

	for index in indexes {
		if !rindexes.contains(&index) {
			eprintln!("removing {:?}", index);
			remove_file(&index).or(Err(format!("Cannot remove {:?}", index)))?;
		};
	}

	Ok(())
}

fn remove(paths: &Vec<PathBuf>) -> Result<(), String> {
	for path in paths {
		if !(path.extension().is_some() && path.extension().unwrap() == "decomposed") {
			return Err(format!("{:?} is not decomposed index", path));
		};
		if !path.exists() {
			return Err(format!("{:?} doesn't exists", path));
		};

		remove_file(&path).or(Err(format!("Cannot remove {:?}", path)))?;
	}

	cleanup()?;

	Ok(())
}

enum Action {
	Create,
	Restore,
	CheckSum,
	Remove,
	Cleanup,
}

fn run() -> Result<(), String> {
	let args = args().skip(1);
	if args.len() == 0 {
		eprintln!("usage: bin_decompose [--restore [--prefix=string] [--postfix=string]] [...file_path > 0]");
		exit(1);
	};
	let mut action: OnceOption<Action> = OnceOption::new();
	let mut prefix = "".to_string();
	let mut postfix = "".to_string();
	let mut paths: Vec<PathBuf> = vec![];
	for arg in args {
		match arg.as_ref() {
			"--restore" => {
				action
					.set(Action::Restore)
					.or(Err("Cannot set action more than one time".to_string()))?;
			}
			"--sha" => {
				action
					.set(Action::CheckSum)
					.or(Err("Cannot set action more than one time".to_string()))?;
			}
			"--remove" => {
				action
					.set(Action::Remove)
					.or(Err("Cannot set action more than one time".to_string()))?;
			}
			"--cleanup" => {
				action
					.set(Action::Cleanup)
					.or(Err("Cannot set action more than one time".to_string()))?;
			}
			x if x.len() >= 9 && &x[0..9] == "--prefix=" => {
				prefix = x[9..].to_string();
			}
			x if x.len() >= 10 && &x[0..10] == "--postfix=" => {
				postfix = x[10..].to_string();
			}
			x => paths.push(PathBuf::from(x)),
		};
	}

	match action.or_default(Action::Create).unwrap() {
		Action::Create => {
			decompose_file(&paths)?;
		}
		Action::Restore => {
			restore_file(&paths, &prefix, &postfix)?;
		}
		Action::CheckSum => {
			output_shasum(&paths)?;
		}
		Action::Remove => {
			remove(&paths)?;
		}
		Action::Cleanup => {
			cleanup()?;
		}
	};

	Ok(())
}

fn main() {
	if let Err(e) = run() {
		eprintln!("{}", e);
		exit(1);
	}
}
