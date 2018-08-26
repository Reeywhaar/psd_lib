use std::cmp::max;
use std::io::{BufReader, Error, Read, Result as IOResult};
use std::mem::transmute_copy;

pub fn u16_to_u8_be_vec<'a>(n: &u16) -> [u8; 2] {
	let bytes: [u8; 2] = unsafe { transmute_copy::<u16, [u8; 2]>(&n.to_be()) };
	bytes
}

pub fn u32_to_u8_be_vec<'a>(n: &u32) -> [u8; 4] {
	let bytes: [u8; 4] = unsafe { transmute_copy::<u32, [u8; 4]>(&n.to_be()) };
	bytes
}

pub fn u64_to_u8_be_vec(n: &u64) -> [u8; 8] {
	let bytes: [u8; 8] = unsafe { transmute_copy::<u64, [u8; 8]>(&n.to_be()) };
	bytes
}

pub fn vec_to_usize_be(input: &[u8]) -> usize {
	let mut o: usize = 0;
	let len = input.len();
	for i in 0..len {
		let shift = len - i - 1;
		let s = (input[i] as usize) << (shift * 8);
		o = o | s;
	}
	return o;
}

pub fn vec_to_u32_be(input: &[u8]) -> u32 {
	let mut o: u32 = 0;
	let len = input.len();
	for i in 0..len {
		let shift = len - i - 1;
		let s = (input[i] as u32) << (shift * 8);
		o = o | s;
	}
	return o;
}

pub fn vec_to_usize_le(input: &[u8]) -> usize {
	let mut o: usize = 0;
	let len = input.len();
	for i in 0..len {
		let s = (input[i] as usize) << (i * 8);
		o = o | s;
	}
	return o;
}

pub fn vec_to_i16_be(n: &[u8]) -> i16 {
	let n = vec_to_usize_be(n);
	let o = unsafe { transmute_copy::<usize, i16>(&n) };
	return o;
}

pub fn u_to_i16_be(n: u16) -> i16 {
	let o = unsafe { transmute_copy::<u16, i16>(&n) };
	return o;
}

pub fn read_usize_be<T: Read>(input: &mut T, size: usize) -> Result<usize, Error> {
	let mut buf = vec![0u8; size];
	input.read_exact(&mut buf)?;
	return Ok(vec_to_usize_be(&buf));
}

pub fn read_usize_le<T: Read>(input: &mut T, size: usize) -> Result<usize, Error> {
	let mut buf = vec![0u8; size];
	input.read_exact(&mut buf)?;
	return Ok(vec_to_usize_le(&buf));
}

/// used to compare T: Read
pub fn cmp_read<'a, T: Read>(
	a: &'a mut T,
	b: &'a mut T,
	buffer_size_in_kb: usize,
) -> IOResult<bool> {
	let mut a = BufReader::with_capacity(1024 * buffer_size_in_kb, a);
	let mut b = BufReader::with_capacity(1024 * buffer_size_in_kb, b);
	let mut buf_a = vec![0; 1024 * buffer_size_in_kb];
	loop {
		let read_a = a.read(&mut buf_a)?;
		let mut buf_b = vec![0; max(read_a, 1)];
		let read_b = b.read_exact(&mut buf_b);
		if read_a == 0 && read_b.is_err() {
			return Ok(true);
		};
		if read_a == 0 && !read_b.is_err() {
			return Ok(false);
		};
		if read_b.is_err() {
			return Ok(false);
		}
		if read_a == 0 {
			return Ok(true);
		};
		if buf_a[..read_a] != buf_b[..] {
			return Ok(false);
		};
	}
}

/// for comparation of medium chunks with buffer of 64kb, for comparation of small chunks use cmp_read_small
pub fn cmp_read_medium<'a, T: Read>(mut a: &'a mut T, mut b: &'a mut T) -> IOResult<bool> {
	return cmp_read(&mut a, &mut b, 64);
}

/// used to compare small chunks, buffer is 1kb
pub fn cmp_read_small<'a, T: Read>(mut a: &'a mut T, mut b: &'a mut T) -> IOResult<bool> {
	return cmp_read(&mut a, &mut b, 1);
}

#[cfg(test)]
mod functions_tests {
	use super::*;
	use std::io::Cursor;

	#[test]
	fn u16_to_u8_be_vec_test() {
		let b = u16_to_u8_be_vec(&10u16);
		eprintln!("{:?}", b);
		assert_eq!(b, [0x00, 10]);
	}

	#[test]
	fn u32_to_u8_be_vec_test() {
		let b = u32_to_u8_be_vec(&10u32);
		eprintln!("{:?}", b);
		assert_eq!(b, [0x00, 0x00, 0x00, 10]);
	}

	#[test]
	fn u64_to_u8_be_vec_test() {
		assert_eq!(
			u64_to_u8_be_vec(&10u64),
			[0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 10]
		);
	}

	#[test]
	fn vec_to_i16_be_test() {
		assert_eq!(vec_to_i16_be(&[0b0000_0001]), 1);
		assert_eq!(vec_to_i16_be(&[0b1000_0000, 0b0000_0000]), -32768);
		assert_eq!(vec_to_i16_be(&[0b1111_1111, 0b1111_1111]), -1);
	}

	#[test]
	fn vec_to_u32_be_test() {
		assert_eq!(vec_to_u32_be(&[0x00, 0x00, 0x00, 0x10]), 16);
	}

	#[test]
	fn cmp_read_test() {
		let mut a = Cursor::new(vec![1, 2, 3, 4, 5, 6, 7, 8]);
		let mut b = Cursor::new(vec![1, 2, 3, 4, 5, 6, 7, 8]);
		assert_eq!(cmp_read_small(&mut a, &mut b).unwrap(), true);

		let mut a = Cursor::new(vec![1, 2, 3, 4, 5, 6, 7, 8]);
		let mut b = Cursor::new(vec![1, 2, 3, 4, 5, 6, 7, 10]);
		assert_eq!(cmp_read_small(&mut a, &mut b).unwrap(), false);

		let mut a = Cursor::new(vec![1, 2, 3, 4, 5, 6, 7, 8]);
		let mut b = Cursor::new(vec![1, 2, 3, 4, 5, 6, 7, 8, 9]);
		assert_eq!(cmp_read_small(&mut a, &mut b).unwrap(), false);

		let mut a = Cursor::new(vec![1, 2, 3, 4, 5, 6, 7, 8]);
		let mut b = Cursor::new(vec![1, 2, 3, 4, 5, 6, 7]);
		assert_eq!(cmp_read_small(&mut a, &mut b).unwrap(), false);
	}
}
