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
//! $: psd_decompose --size [--as-bytes] [...file.psd.decomposed > 1]
//!    Works in two modes:
//!    * first: if all of the paths is decomposed object files, then it calculates presumable size of decompressed files
//!    * second: calculates size of prospective "decomposed_objects" directory and outputs it's next to accumulated size of given paths, which shows is it worth to decompose files
//!
//!    --as-bytes: output size in bytes instead of human readable version
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
extern crate num_cpus;
extern crate psd_lib;
extern crate sha2;
extern crate threadpool;

mod once_option;

use bin_diff::lines_with_hash_iterator::LinesWithHashIterator;
use once_option::OnceOption;
use psd_lib::psd_file::PSDFile;
use sha2::{Digest, Sha256};
use std::env::args;
use std::fs::{create_dir_all, metadata, read_dir, remove_file, File};
use std::io::{copy, BufRead, BufReader, BufWriter, Read, Seek, SeekFrom, Write};
use std::path::PathBuf;
use std::process::exit;
use std::sync::mpsc::channel;
use std::sync::{Arc, Mutex};
use std::thread::sleep as thread_sleep;
use std::time::Duration;
use threadpool::ThreadPool;

const EMPTY_HASH: &str = "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855";

fn bytes_to_human_readable(size: u64) -> String {
	let mask = 0b1111111111;
	let gb = (size >> 10 * 3) & mask;
	let mb = (size >> 10 * 2) & mask;
	let kb = (size >> 10 * 1) & mask;
	let b = size & mask;
	return format!("{}GB {}MB {}KB {}B", gb, mb, kb, b);
}

fn get_objects<'a, T: 'a + Read + Seek>(
	input: T,
) -> Result<LinesWithHashIterator<PSDFile<T>>, String> {
	let file = PSDFile::new(input);
	return LinesWithHashIterator::new(file);
}

fn get_objects_parallel(
	paths: &Vec<PathBuf>,
) -> Result<Vec<Vec<(String, u64, u64, String)>>, String> {
	let mut out = vec![vec![]; paths.len()];
	let pool = ThreadPool::new(num_cpus::get());
	let (tx, rx) = channel();
	for (index, path) in paths.iter().enumerate() {
		let tx = tx.clone();
		let path = path.clone();
		pool.execute(move || {
			let mut input = match File::open(&path).or(Err(format!("Cannot open {:?}", path))) {
				Ok(i) => i,
				Err(e) => {
					tx.send(Err(e)).unwrap();
					return;
				}
			};
			let obj = match get_objects(&mut input)
				.map_err(|e| format!("path: {}\nerror: {}", path.display(), e))
			{
				Ok(data) => data,
				Err(e) => {
					tx.send(Err(e)).unwrap();
					return;
				}
			};
			tx.send(Ok((index, obj.collect::<Vec<_>>()))).unwrap();
		});
	}

	for _ in 0..paths.len() {
		if pool.panic_count() > 0 {
			return Err("Failed getting objects".to_string());
		};
		let (index, obj) = rx.recv().map_err(|e| e.to_string())??;
		out[index] = obj;
	}

	Ok(out)
}

fn decompose_files(paths: &Vec<PathBuf>) -> Result<(), String> {
	let objects_store = get_objects_parallel(&paths)?;

	for (index, path) in paths.iter().enumerate() {
		let mut input = File::open(&path).or(Err(format!("Cannot open {:?}", path)))?;
		let objects = &objects_store[index];

		eprintln!("processing {:?}", path);

		let objdir = path.parent().ok_or("Parent directory not found")?;
		let mut objdir = PathBuf::from(objdir);
		objdir.push("decomposed_objects");

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

		for (_label, start, size, hash) in objects.iter() {
			if hash == EMPTY_HASH {
				continue;
			};

			let mut hashdir = objdir.clone();
			hashdir.push(&hash[0..2]);
			let mut hashloc = hashdir.clone();
			hashloc.push(hash);

			if hashloc.exists() {
				indexfile
					.write(format!("{}\n", hash).as_bytes())
					.or(Err(format!(
						"Cannot write to index file: {:?}",
						indexfileloc
					)))?;
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

fn restore_files(paths: &Vec<PathBuf>, prefix: &str, postfix: &str) -> Result<(), String> {
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

fn get_unique_hashes(path: &PathBuf) -> Result<Vec<(String, u64)>, String> {
	if path.extension().is_some() && path.extension().unwrap() == "decomposed" {
		let file = File::open(&path).or(Err(format!("Cannot open path: {:?}", path)))?;
		let file = BufReader::with_capacity(1024, file);

		let objdir = path
			.parent()
			.ok_or("Parent directory not found".to_string())?;
		let mut objdir = PathBuf::from(objdir);
		objdir.push("decomposed_objects");

		let mut output: Vec<(String, u64)> = vec![];

		for hash in file.lines() {
			let hash = hash.map_err(|e| e.to_string())?;

			if hash == EMPTY_HASH {
				output.push((hash, 0));
				continue;
			}

			let mut hashdir = objdir.clone();
			hashdir.push(&hash[0..2]);
			let mut hashloc = hashdir.clone();
			hashloc.push(hash.clone());

			if !hashloc.exists() {
				return Err(format!("hash {:?} doesn't exists", hashloc));
			}

			let hash_meta = metadata(&hashloc).map_err(|e| e.to_string())?;
			output.push((hash, hash_meta.len()));
		}

		return Ok(output);
	};

	let mut input = File::open(&path).or(Err(format!("Cannot open {:?}", path)))?;
	let hashes = {
		let obj = get_objects(&mut input)?;
		obj.map(|x| (x.3, x.2)).fold(vec![], |mut c, x| {
			if !c.contains(&x) {
				c.push(x);
			};
			return c;
		})
	};

	return Ok(hashes);
}

#[derive(Clone)]
enum CalcMode {
	Composed,
	Decomposed,
}

fn calc_size(paths: &Vec<PathBuf>, as_bytes: bool) -> Result<(), String> {
	let write_lock = Arc::new(Mutex::new(()));
	let total_hashes: Arc<Mutex<Vec<(String, u64)>>> = Arc::new(Mutex::new(vec![]));
	let pool = ThreadPool::new(num_cpus::get());
	let (tx, rx) = channel();

	let mode = {
		let decomposed_mode = paths.into_iter().all(|x| {
			return x.extension().is_some() && x.extension().unwrap() == "decomposed";
		});

		let composed_mode = paths.into_iter().all(|x| {
			return x.extension().is_some() && x.extension().unwrap() != "decomposed";
		});

		match (decomposed_mode, composed_mode) {
			(true, false) => CalcMode::Decomposed,
			(false, true) => CalcMode::Composed,
			_ => {
				return Err("Invalid mode".to_string());
			}
		}
	};

	for path in paths {
		let path = path.clone();
		let wr_clone = Arc::clone(&write_lock);
		let th_clone = Arc::clone(&total_hashes);
		let tx = tx.clone();
		let mode = mode.clone();
		pool.execute(move || {
			let hashes = get_unique_hashes(&path);
			if hashes.is_err() {
				tx.send(hashes.map(|_| ())).unwrap();
				return;
			};
			let hashes = hashes.unwrap();

			let wr_lock = wr_clone.lock().unwrap();

			let size = hashes.iter().fold(0u64, |c, x| c + (x.1 as u64));
			if as_bytes {
				println!("{} - {}", path.display(), size);
			} else {
				println!("{} - {}", path.display(), bytes_to_human_readable(size));
			};

			drop(wr_lock);

			let mut total_hashes = th_clone.lock().unwrap();

			if let CalcMode::Composed = mode {
				hashes.into_iter().for_each(|x| {
					if !total_hashes.contains(&x) {
						total_hashes.push(x);
					};
				});
			} else {
				hashes.into_iter().for_each(|x| {
					total_hashes.push(x);
				});
			};

			tx.send(Ok(())).unwrap();
		});
	}

	for _path in paths {
		rx.recv().unwrap()?;
	}

	if let CalcMode::Composed = mode {
		let filesize = paths
			.iter()
			.map(|x| {
				let meta = metadata(x).unwrap();
				meta.len()
			})
			.fold(0u64, |c, x| c + x);

		let size = total_hashes
			.lock()
			.unwrap()
			.iter()
			.fold(0u64, |c, x| c + (x.1 as u64));

		if as_bytes {
			println!("\ntotal size         - {}", filesize);
			println!("decomposed_objects - {}", size);
		} else {
			println!(
				"\ntotal size         - {}",
				bytes_to_human_readable(filesize)
			);
			println!("decomposed_objects - {}", bytes_to_human_readable(size));
		};
	} else {
		let size = total_hashes
			.lock()
			.unwrap()
			.iter()
			.fold(0u64, |c, x| c + (x.1 as u64));

		if as_bytes {
			println!("\ntotal size - {}", size);
		} else {
			println!("\ntotal size - {}", bytes_to_human_readable(size));
		};
	}

	Ok(())
}

fn get_shasum(path: &PathBuf) -> Result<String, String> {
	let file = File::open(&path).or(Err(format!("Cannot open path: {:?}", path)))?;
	let mut file = BufReader::with_capacity(1024, file);

	let buf = &mut [0u8; 1024 * 64];
	let mut hasher = Sha256::default();

	if path.extension().is_some() && path.extension().unwrap() == "decomposed" {
		let objdir = path.parent().expect("Parent directory not found");
		let mut objdir = PathBuf::from(objdir);
		objdir.push("decomposed_objects");

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
					.read(buf)
					.or(Err(format!("Cannot read decomposed chunk {:?}", &hashloc)))?;
				if read == 0 {
					break;
				};
				hasher.input(&buf[..read]);
			}
		}
	} else {
		loop {
			let read = file
				.read(buf)
				.or(Err(format!("Cannot read file {:?}", &path)))?;
			if read == 0 {
				break;
			};
			hasher.input(&buf[..read]);
		}
	}

	return Ok(hasher
		.result()
		.iter()
		.map(|b| format!("{:02x}", b))
		.collect::<Vec<String>>()
		.join(""));
}

fn output_shasum(paths: &Vec<PathBuf>) -> Result<(), String> {
	let paths_len = paths.len();
	let hashes = Arc::new(Mutex::new(vec!["".to_string(); paths.len()]));
	let pool = ThreadPool::new(num_cpus::get());
	for (index, path) in paths.iter().enumerate() {
		let path = path.clone();
		let hp = Arc::clone(&hashes);
		pool.execute(move || {
			let sum = get_shasum(&path).unwrap();
			let mut hashes = hp.lock().unwrap();
			hashes[index] = sum;
		});
	}

	let mut i = 0;
	while i < paths_len {
		if pool.panic_count() > 0 {
			return Err("program failed".to_string());
		};
		let hashes = hashes.lock().unwrap();
		if hashes[i] == "" {
			drop(hashes);
			thread_sleep(Duration::from_millis(200));
			continue;
		}
		println!("{} - {}", hashes[i], paths[i].display());
		i += 1;
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
		.or(Err("Cannot read decomposed_objects directory"))?
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

#[derive(PartialEq, Eq, Clone)]
enum Action {
	Create,
	Restore,
	Size,
	CheckSum,
	Remove,
	Cleanup,
}

fn run() -> Result<(), String> {
	let usage_str = "\
$: psd_decompose [...file.psd > 1]

$: psd_decompose --restore [--prefix=string] [--postfix=string] [...file.psd.decomposed > 1]
   --prefix:  prepend string to restored filename
   --postfix: append string to restored filename before extension

$: psd_decompose --size [--as-bytes] [...file.psd.decomposed > 1]
   * first: if all of the paths are .decomposed files, then it calculates presumable size of decompressed files
   * second: calculates size of prospective \"decomposed_objects\" directory and outputs it's next to accumulated size of given paths, which shows is it worth to decompose files

   --as-bytes: output size in bytes instead of human readable version

$: psd_decompose --sha [...file > 1]
   compute sha256 hash of given prospective restored files or ordinary files. Usefull to check that restore will be correct.

$: psd_decompose --remove [...file.decomposed > 1]
   removes decomposed index file and rebuild (actually gather all the hashes from other files in the directory and removes hashes which are orphaned) decomposed_opjects directory.

$: psd_decompose --cleanup
   performs cleanup of \"decomposed_objects\" directory which consists of populating unique index of every hash of every .decomposed file and removing every hash which doesn't said index contains.
	";

	let args = args().skip(1);
	if args.len() == 0 {
		eprintln!("{}", usage_str);
		exit(1);
	};
	let mut action: OnceOption<Action> = OnceOption::new();
	let mut prefix = "".to_string();
	let mut postfix = "".to_string();
	let mut as_bytes = false;
	let mut paths: Vec<PathBuf> = vec![];
	for arg in args {
		match arg.as_ref() {
			"--restore" => {
				action
					.set(Action::Restore)
					.or(Err("Cannot set action more than one time".to_string()))?;
			}
			"--size" => {
				action
					.set(Action::Size)
					.or(Err("Cannot set action more than one time".to_string()))?;
			}
			x if x == "--as-bytes" && *action == Some(Action::Size) => {
				as_bytes = true;
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
			x if x.len() >= 9 && &x[0..9] == "--prefix=" && *action == Some(Action::Restore) => {
				prefix = x[9..].to_string();
			}
			x if x.len() >= 10 && &x[0..10] == "--postfix=" && *action == Some(Action::Restore) => {
				postfix = x[10..].to_string();
			}
			x => paths.push(PathBuf::from(x)),
		};
	}

	match action.or_default(Action::Create) {
		Action::Create => {
			decompose_files(&paths)?;
		}
		Action::Restore => {
			restore_files(&paths, &prefix, &postfix)?;
		}
		Action::Size => {
			calc_size(&paths, as_bytes)?;
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
