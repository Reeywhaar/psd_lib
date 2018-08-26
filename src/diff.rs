extern crate difference;
extern crate sha2;

use self::difference::{Changeset, Difference};
use self::sha2::{Digest, Sha256};
use bytes_serializer::IntoBytesSerializer;
use common::get_lines;
use diffblock::{DiffBlock, DiffBlockN};
use functions::vec_to_u32_be;
use std::io::{
	copy, sink, BufWriter, Error, ErrorKind, Read, Result as IOResult, Seek, SeekFrom, Take, Write,
};
use std::str;

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

pub struct LinesWithHashIterator<T: Read + Seek> {
	file: T,
	indexes: Box<Vec<(String, u64, u64)>>,
	pos: usize,
}

impl<T: Read + Seek> LinesWithHashIterator<T> {
	pub fn new(mut file: T) -> Result<Self, String> {
		let indexes = get_lines(&mut file)?;
		let indexes = Box::new(indexes);
		return Ok(Self {
			file: file,
			indexes: indexes,
			pos: 0,
		});
	}

	pub fn get_read(self) -> T {
		return self.file;
	}
}

impl<T: Read + Seek> Iterator for LinesWithHashIterator<T> {
	type Item = (String, u64, u64, String);

	fn next(&mut self) -> Option<Self::Item> {
		if self.pos >= self.indexes.len() {
			return None;
		};
		let item = (&self.indexes[self.pos]).clone();
		self.file.seek(SeekFrom::Start(item.1)).unwrap();
		let hash = {
			let mut sl = &mut self.file.by_ref().take(item.2);
			compute_hash(&mut sl)
		};

		self.pos += 1;
		return Some((item.0, item.1, item.2, hash));
	}
}

pub struct DiffIterator<T: Read + Seek> {
	file: T,
	diff: Vec<DiffBlockN<u32>>,
	pos: usize,
	file_pos: u64,
}

impl<T: Seek + Read> DiffIterator<T> {
	pub fn new<U: Read + Seek>(file_a: U, file_b: T) -> Result<Self, String> {
		let (_file_a, ind_a) = {
			let mut it = LinesWithHashIterator::new(file_a)?;
			let ind: Vec<_> = it.by_ref().collect();
			let r = it.get_read();
			(r, ind.clone())
		};
		let (file_b, ind_b) = {
			let mut it = LinesWithHashIterator::new(file_b)?;
			let ind: Vec<_> = it.by_ref().collect();
			let r = it.get_read();
			(r, ind.clone())
		};

		let ind_a_h = (&ind_a)
			.into_iter()
			.by_ref()
			.map(|x| x.3.clone())
			.collect::<Vec<String>>()
			.join("\n");
		let ind_b_h = (&ind_b)
			.into_iter()
			.by_ref()
			.map(|x| x.3.clone())
			.collect::<Vec<String>>()
			.join("\n");

		let diffs = {
			let changeset = Changeset::new(&ind_a_h, &ind_b_h, "\n");
			changeset.diffs
		};

		let diffs = Self::process_diff(&diffs);
		let diffs = Self::process_diff_2(&diffs, &ind_a, &ind_b);

		return Ok(Self {
			file: file_b,
			diff: diffs,
			pos: 0,
			file_pos: 0,
		});
	}

	fn process_diff(diffs: &Vec<Difference>) -> Vec<DiffBlockN<usize>> {
		let mut o: Vec<DiffBlockN<usize>> = vec![DiffBlockN::Skip(0)];

		for d in diffs {
			match d {
				Difference::Same(x) => {
					let blocks_n = x.matches("\n").count() + 1;
					let last_item = o[o.len() - 1].clone();
					if let DiffBlockN::Skip(n) = last_item {
						let last_index = o.len() - 1;
						o[last_index] = DiffBlockN::Skip(n + blocks_n);
					} else {
						o.push(DiffBlockN::Skip(blocks_n));
					};
				}
				Difference::Rem(x) => {
					o.push(DiffBlockN::Remove(x.matches("\n").count() + 1));
				}
				Difference::Add(x) => {
					let blocks_n = x.matches("\n").count() + 1;
					let last_item = o[o.len() - 1].clone();
					if let DiffBlockN::Remove(n) = last_item {
						let last_index = o.len() - 1;
						o[last_index] = DiffBlockN::Replace(n, blocks_n);
					} else {
						o.push(DiffBlockN::Add(blocks_n));
					};
				}
			}
		}

		return o;
	}

	fn process_diff_2(
		diffs: &Vec<DiffBlockN<usize>>,
		indexes_a: &Vec<(String, u64, u64, String)>,
		indexes_b: &Vec<(String, u64, u64, String)>,
	) -> Vec<DiffBlockN<u32>> {
		let mut o: Vec<DiffBlockN<u32>> = vec![];
		let mut i_a = indexes_a.into_iter().map(|x| x.2 as u32);
		let mut i_b = indexes_b.into_iter().map(|x| x.2 as u32);

		for item in diffs {
			match item {
				DiffBlockN::Skip(n) => {
					let size = (&mut i_a).by_ref().take(*n).fold(0, |acc, x| acc + x);
					let _: Vec<_> = (&mut i_b).by_ref().take(*n).collect();
					if size != 0 {
						o.push(DiffBlockN::Skip(size));
					}
				}
				DiffBlockN::Add(n) => {
					let size = (&mut i_b).by_ref().take(*n).fold(0, |acc, x| acc + x);
					if size != 0 {
						o.push(DiffBlockN::Add(size));
					}
				}
				DiffBlockN::Remove(n) => {
					let size = (&mut i_a).by_ref().take(*n).fold(0, |acc, x| acc + x);
					if size != 0 {
						o.push(DiffBlockN::Remove(size));
					}
				}
				DiffBlockN::Replace(r, a) => {
					let remove = (&mut i_a).by_ref().take(*r).fold(0, |acc, x| acc + x);
					let add = (&mut i_b).by_ref().take(*a).fold(0, |acc, x| acc + x);
					if remove != 0 && add != 0 {
						if add == remove {
							o.push(DiffBlockN::ReplaceWithSameLength(add));
						} else {
							o.push(DiffBlockN::Replace(remove, add));
						}
					} else if remove != 0 {
						o.push(DiffBlockN::Remove(remove));
					} else if add != 0 {
						o.push(DiffBlockN::Add(add));
					}
				}
				_ => panic!("Strange situation when process_diff returns unidentifiable block"),
			}
		}

		return o;
	}

	pub fn next_ref<'a>(&mut self) -> Option<Result<DiffBlock<u32, Take<&mut T>>, String>> {
		if self.pos >= self.diff.len() {
			return None;
		};

		let item = &self.diff[self.pos];
		self.pos += 1;

		match item {
			DiffBlockN::Skip(size) => {
				self.file_pos += *size as u64;
				return Some(Ok(DiffBlock::Skip { size: *size }));
			}
			DiffBlockN::Add(size) => {
				let res = self.file.seek(SeekFrom::Start(self.file_pos));
				if res.is_err() {
					return Some(Err("Error while seeking file".to_string()));
				};
				let slice = self.file.by_ref().take(*size as u64);
				self.file_pos += *size as u64;
				return Some(Ok(DiffBlock::Add {
					size: *size as u32,
					data: slice,
				}));
			}
			DiffBlockN::Remove(size) => {
				return Some(Ok(DiffBlock::Remove { size: *size }));
			}
			DiffBlockN::Replace(remove, add) => {
				let res = self.file.seek(SeekFrom::Start(self.file_pos));
				if res.is_err() {
					return Some(Err("Error while seeking file".to_string()));
				};
				let slice = self.file.by_ref().take(*add as u64);
				self.file_pos += *add as u64;
				return Some(Ok(DiffBlock::Replace {
					replace_size: *remove,
					size: *add,
					data: slice,
				}));
			}
			DiffBlockN::ReplaceWithSameLength(size) => {
				let res = self.file.seek(SeekFrom::Start(self.file_pos));
				if res.is_err() {
					return Some(Err("Error while seeking file".to_string()));
				};
				let slice = self.file.by_ref().take(*size as u64);
				self.file_pos += *size as u64;
				return Some(Ok(DiffBlock::ReplaceWithSameLength {
					size: *size,
					data: slice,
				}));
			}
		}
	}
}

#[cfg(test)]
mod diff_block_tests {
	use super::DiffIterator;
	use std::fs::File;

	#[test]
	fn diffiterator_test() {
		let file_a = File::open("./test_data/a_a.psd").unwrap();
		let file_b = File::open("./test_data/a_b.psd").unwrap();
		let mut it = DiffIterator::new(file_a, file_b).unwrap();
		let mut i = 0;
		while let Some(_block) = it.next_ref() {
			i += 1;
		}
		assert_eq!(i, 18);
	}
}

pub fn create_diff<T: Read + Seek, U: Read + Seek, W: Write>(
	original: &mut T,
	edited: &mut U,
	output: &mut W,
) -> IOResult<()> {
	let mut dit = DiffIterator::new(original, edited).or(Err(Error::new(
		ErrorKind::Other,
		"Error while creating DiffIterator",
	)))?;

	let mut stdo = BufWriter::with_capacity(1024 * 64, output);

	stdo.write("PSDDIFF1".as_bytes())
		.or(Err(Error::new(ErrorKind::Other, "Cannot write signature")))?;
	stdo.write(&[0x00, 0x01])
		.or(Err(Error::new(ErrorKind::Other, "Cannot write version")))?;

	let mut buf = vec![0u8; 1024 * 64];
	while let Some(block) = dit.next_ref() {
		let mut block = block
			.or(Err(Error::new(ErrorKind::Other, "Cannot get diff block")))
			.map(|x| x.into_bytes())?;
		loop {
			let x = block.read(&mut buf)?;
			if x == 0 {
				break;
			}
			stdo.write(&buf[0..x])?;
		}
	}
	stdo.flush()?;
	Ok(())
}

pub fn apply_diff<T: Read, U: Read, W: Write>(
	mut file: &mut T,
	mut diff: &mut U,
	mut output: &mut W,
) -> IOResult<()> {
	let mut buf = vec![0; 1024 * 64];
	{
		(&mut diff).take(8).by_ref().read(&mut buf)?;
		if str::from_utf8(&buf[0..8]).unwrap() != "PSDDIFF1" {
			return Err(Error::new(ErrorKind::Other, "Signature mismatch"));
		};
	}
	{
		(&mut diff).take(2).by_ref().read(&mut buf)?;
		if &buf[0..2] != [0x00, 0x01] {
			return Err(Error::new(ErrorKind::Other, "Version mismatch"));
		};
	};
	let mut output = BufWriter::with_capacity(8, &mut output);
	let mut sink = sink();
	let mut drain = |mut input: &mut T, size: u32| -> IOResult<()> {
		let mut r = (&mut input).take(size as u64);
		while copy(&mut r, &mut sink)? != 0 {}
		return Ok(());
	};
	let read = |mut input: &mut U, buf: &mut [u8], size: u32| -> IOResult<usize> {
		let mut taken = (&mut input).take(size as u64);
		let mut read: usize = 0;
		let mut attempts = 0;
		while read < size as usize {
			let r = taken.read(&mut buf[read..])?;
			read += r;
			if r == 0 {
				attempts += 1;
				if attempts >= 10 {
					return Err(Error::new(ErrorKind::UnexpectedEof, "Unexpected EOF"));
				}
			} else {
				attempts = 0;
			}
		}
		Ok(read)
	};

	loop {
		let res = read(&mut diff, &mut buf, 2);

		if res.is_err() {
			break;
		}

		let slice: &[u8] = &buf[0..2].to_vec();
		match slice.as_ref() {
			[0x00, 0x00] => {
				read(&mut diff, &mut buf, 4)?;
				let len = vec_to_u32_be(&buf[0..4]);
				let mut r = (&mut file).take(len as u64);
				copy(&mut r, &mut output)?;
			}
			[0x00, 0x01] => {
				read(&mut diff, &mut buf, 4)?;
				let len = vec_to_u32_be(&buf[0..4]);
				let mut r = (&mut diff).take(len as u64);
				copy(&mut r, &mut output)?;
			}
			[0x00, 0x02] => {
				read(&mut diff, &mut buf, 4)?;
				let len = vec_to_u32_be(&buf[0..4]);
				drain(&mut file, len)?;
			}
			[0x00, 0x03] => {
				read(&mut diff, &mut buf, 4)?;
				let remove = vec_to_u32_be(&buf[0..4]);
				read(&mut diff, &mut buf, 4)?;
				let add = vec_to_u32_be(&buf[0..4]);
				drain(&mut file, remove)?;
				let mut r = (&mut diff).take(add as u64);
				copy(&mut r, &mut output)?;
			}
			[0x00, 0x04] => {
				read(&mut diff, &mut buf, 4)?;
				let size = vec_to_u32_be(&buf[0..4]);
				drain(&mut file, size)?;
				let mut r = (&mut diff).take(size as u64);
				copy(&mut r, &mut output)?;
			}
			_ => {
				return Err(Error::new(
					ErrorKind::Other,
					"Unknown Action: possibly corrupted file or diff",
				));
			}
		}
	}
	return output.flush();
}

#[cfg(test)]
mod apply_diff_tests {
	use super::{apply_diff, compute_hash, create_diff};
	use std::fs::File;
	use std::io::{Cursor, Seek, SeekFrom};

	#[test]
	fn works_test() {
		#[cfg_attr(rustfmt, rustfmt_skip)]
		let mut file = Cursor::new(vec![
			0xd0, 0x4b, 0x51, 0x00, 0x25, 0xb6, 0x95, 0xf3,
			0xb0, 0xa9, 0x59, 0xdc, 0x30, 0x35, 0x16, 0x7d,
			0x06, 0xa1, 0xf7, 0x66, 0x64, 0x33, 0x05, 0xee,
			0x2b, 0x35, 0xa9, 0x38, 0x80, 0x7f, 0x1c, 0x90,
			0x2c, 0x29, 0x2a, 0x49, 0x79, 0x66, 0x83, 0x55,
			0x8e, 0xce, 0x78, 0xd4, 0xef, 0x0f, 0xaa, 0xaa,
			0x1c, 0x41, 0xaf, 0xa2, 0xed, 0x85, 0xb6, 0x16,
			0x22, 0xe5, 0x83, 0x7a, 0xf7, 0x73, 0x78, 0xf5,
			0xf5, 0x63, 0x3b, 0x0a, 0x6d, 0xe5, 0x0b, 0x36,
			0x4b, 0x97, 0xc2, 0xfe, 0x84, 0x40, 0x3f, 0x74,
			0x20, 0x4b, 0xbb, 0xfe, 0x4c, 0xe1, 0x87, 0xc2,
			0x55, 0x71, 0xa3, 0x87, 0x55, 0xad, 0x87, 0xad,
			0x08, 0x69, 0x39, 0x0f, 0x8d, 0xe2, 0x9a, 0xef,
		]);

		#[cfg_attr(rustfmt, rustfmt_skip)]
		let mut diff = Cursor::new(vec![
			0x50, 0x53, 0x44, 0x44, 0x49, 0x46, 0x46, 0x31, // PSDDIFF1
			0x00, 0x01, // version
			0x00, 0x00, 0x00, 0x00, 0x00, 0x10, // skip 16
			0x00, 0x01, 0x00, 0x00, 0x00, 0x20, // add 32
			0xef, 0x22, 0xe4, 0x2c, 0x5f, 0x3c, 0xde, 0x10, //
			0x8d, 0x27, 0x6c, 0xdd, 0xbc, 0xc6, 0xff, 0xf9, //
			0x5c, 0xe1, 0x81, 0x53, 0xda, 0x3b, 0xa6, 0x7e, //
			0xa9, 0xee, 0xe0, 0x00, 0x67, 0x24, 0x25, 0x78, // added 32 data
			0x00, 0x00, 0x00, 0x00, 0x00, 0x08, // skip 8
			0x00, 0x02, 0x00, 0x00, 0x00, 0x10, // remove 16
			0x00, 0x00, 0x00, 0x00, 0x00, 0x10, // skip 16
			0x00, 0x03, 0x00, 0x00, 0x00, 0x10, 0x00, 0x00, 0x00, 0x20, // replace 16 with 32
			0x23, 0x2a, 0xe9, 0x85, 0xfa, 0x6d, 0xb6, 0x78, //
			0xcd, 0x55, 0x66, 0xc2, 0x03, 0x80, 0x33, 0x3d, //
			0xc1, 0x8c, 0x62, 0xfb, 0xbb, 0xde, 0xe2, 0x53, //
			0xc7, 0x41, 0x0e, 0x82, 0xff, 0x60, 0x40, 0xf0, // added 32 data
			0x00, 0x00, 0x00, 0x00, 0x00, 0x20, // skip 32
		]);

		#[cfg_attr(rustfmt, rustfmt_skip)]
		let result = vec![
			0xd0, 0x4b, 0x51, 0x00, 0x25, 0xb6, 0x95, 0xf3, //
			0xb0, 0xa9, 0x59, 0xdc, 0x30, 0x35, 0x16, 0x7d, // skipped
			0xef, 0x22, 0xe4, 0x2c, 0x5f, 0x3c, 0xde, 0x10, //
			0x8d, 0x27, 0x6c, 0xdd, 0xbc, 0xc6, 0xff, 0xf9, //
			0x5c, 0xe1, 0x81, 0x53, 0xda, 0x3b, 0xa6, 0x7e, //
			0xa9, 0xee, 0xe0, 0x00, 0x67, 0x24, 0x25, 0x78, // added
			0x06, 0xa1, 0xf7, 0x66, 0x64, 0x33, 0x05, 0xee, // skipped
			// removed 16
			0x8e, 0xce, 0x78, 0xd4, 0xef, 0x0f, 0xaa, 0xaa, //
			0x1c, 0x41, 0xaf, 0xa2, 0xed, 0x85, 0xb6, 0x16, // skipped 16
			// removed 16 and replaced ->
			0x23, 0x2a, 0xe9, 0x85, 0xfa, 0x6d, 0xb6, 0x78, //
			0xcd, 0x55, 0x66, 0xc2, 0x03, 0x80, 0x33, 0x3d, //
			0xc1, 0x8c, 0x62, 0xfb, 0xbb, 0xde, 0xe2, 0x53, //
			0xc7, 0x41, 0x0e, 0x82, 0xff, 0x60, 0x40, 0xf0, //added 32
			0x4b, 0x97, 0xc2, 0xfe, 0x84, 0x40, 0x3f, 0x74, //
			0x20, 0x4b, 0xbb, 0xfe, 0x4c, 0xe1, 0x87, 0xc2, //
			0x55, 0x71, 0xa3, 0x87, 0x55, 0xad, 0x87, 0xad, //
			0x08, 0x69, 0x39, 0x0f, 0x8d, 0xe2, 0x9a, 0xef, // skipped 32
		];

		let mut output = Cursor::new(vec![0, 136]);
		apply_diff(&mut file, &mut diff, &mut output).unwrap();
		assert_eq!(output.get_ref(), &result);
	}

	#[test]
	fn signature_fail_test() {
		#[cfg_attr(rustfmt, rustfmt_skip)]
		let mut file = Cursor::new(vec![
			0xd0, 0x4b, 0x51, 0x00, 0x25, 0xb6, 0x95, 0xf3,
			0xb0, 0xa9, 0x59, 0xdc, 0x30, 0x35, 0x16, 0x7d,
			0x06, 0xa1, 0xf7, 0x66, 0x64, 0x33, 0x05, 0xee,
			0x2b, 0x35, 0xa9, 0x38, 0x80, 0x7f, 0x1c, 0x90,
		]);

		#[cfg_attr(rustfmt, rustfmt_skip)]
		let mut diff = Cursor::new(vec![
			0x50, 0x53, 0x44, 0x44, 0x49, 0x46, 0x46, 0x32, // PSDDIFF2
			0x00, 0x01, // version
			0x00, 0x00, 0x00, 0x00, 0x00, 0x10, // skip 16
			0x00, 0x01, 0x00, 0x00, 0x00, 0x20, // add 32
		]);

		let mut output = Cursor::new(vec![0, 136]);
		let res = apply_diff(&mut file, &mut diff, &mut output);
		assert_eq!(
			res.unwrap_err().to_string(),
			"Signature mismatch".to_string()
		)
	}

	#[test]
	fn version_fail_test() {
		#[cfg_attr(rustfmt, rustfmt_skip)]
		let mut file = Cursor::new(vec![
			0xd0, 0x4b, 0x51, 0x00, 0x25, 0xb6, 0x95, 0xf3,
			0xb0, 0xa9, 0x59, 0xdc, 0x30, 0x35, 0x16, 0x7d,
			0x06, 0xa1, 0xf7, 0x66, 0x64, 0x33, 0x05, 0xee,
			0x2b, 0x35, 0xa9, 0x38, 0x80, 0x7f, 0x1c, 0x90,
		]);

		#[cfg_attr(rustfmt, rustfmt_skip)]
		let mut diff = Cursor::new(vec![
			0x50, 0x53, 0x44, 0x44, 0x49, 0x46, 0x46, 0x31, // PSDDIFF2
			0x00, 0x02, // version
			0x00, 0x00, 0x00, 0x00, 0x00, 0x10, // skip 16
			0x00, 0x01, 0x00, 0x00, 0x00, 0x20, // add 32
		]);

		let mut output = Cursor::new(vec![0, 136]);
		let res = apply_diff(&mut file, &mut diff, &mut output);
		assert_eq!(res.unwrap_err().to_string(), "Version mismatch".to_string())
	}

	#[test]
	fn action_fail_test() {
		#[cfg_attr(rustfmt, rustfmt_skip)]
		let mut file = Cursor::new(vec![
			0xd0, 0x4b, 0x51, 0x00, 0x25, 0xb6, 0x95, 0xf3,
			0xb0, 0xa9, 0x59, 0xdc, 0x30, 0x35, 0x16, 0x7d,
			0x06, 0xa1, 0xf7, 0x66, 0x64, 0x33, 0x05, 0xee,
			0x2b, 0x35, 0xa9, 0x38, 0x80, 0x7f, 0x1c, 0x90,
		]);

		#[cfg_attr(rustfmt, rustfmt_skip)]
		let mut diff = Cursor::new(vec![
			0x50, 0x53, 0x44, 0x44, 0x49, 0x46, 0x46, 0x31, // PSDDIFF2
			0x00, 0x01, // version
			0x4a, 0x00, 0x00, 0x00, 0x00, 0x10, // skip 16
			0x00, 0x01, 0x00, 0x00, 0x00, 0x20, // add 32
		]);

		let mut output = Cursor::new(vec![0, 136]);
		let res = apply_diff(&mut file, &mut diff, &mut output);
		eprintln!("{:?}", res);
		assert_eq!(
			res.unwrap_err().to_string(),
			"Unknown Action: possibly corrupted file or diff".to_string()
		)
	}

	#[test]
	fn works_live_test() {
		#[cfg_attr(rustfmt, rustfmt_skip)]
		let inputs = [
			["a_a.psd", "a_b.psd"],
			["b_a.psd", "b_b.psd"],
			["a_a.psd", "b_b.psd"],
		];

		for pair in inputs.iter() {
			let pairs = [[pair[0], pair[1]], [pair[1], pair[0]]];
			for pair in pairs.iter() {
				let mut file_a = File::open(format!("./test_data/{}", pair[0])).unwrap();
				let mut file_b = File::open(format!("./test_data/{}", pair[1])).unwrap();

				let hash = compute_hash(&mut file_b);
				file_b.seek(SeekFrom::Start(0)).unwrap();

				let mut diff = Cursor::new(vec![]);
				create_diff(&mut file_a, &mut file_b, &mut diff).unwrap();
				diff.seek(SeekFrom::Start(0)).unwrap();

				file_a.seek(SeekFrom::Start(0)).unwrap();
				let mut restored = Cursor::new(vec![]);
				apply_diff(&mut file_a, &mut diff, &mut restored).unwrap();
				restored.seek(SeekFrom::Start(0)).unwrap();

				let res_hash = compute_hash(&mut restored);

				assert_eq!(hash, res_hash, "pair {:?} failed", pair);
			}
		}
	}
}
