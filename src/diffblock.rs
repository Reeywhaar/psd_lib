use bytes_serializer::{BytesSerializer, IntoBytesSerializer};
use functions::{u16_to_u8_be_vec, u32_to_u8_be_vec};
use std::io::{Cursor, Read};
use std::ops::{Add, AddAssign, Sub, SubAssign};

#[derive(Clone, Debug)]
pub enum DiffBlockN<T: Add + AddAssign + Sub + SubAssign> {
	Skip(T),
	Add(T),
	Remove(T),
	Replace(T, T),
	ReplaceWithSameLength(T),
}

#[derive(Clone, Debug)]
pub enum DiffBlock<T, U: Read> {
	Skip { size: T },
	Add { size: T, data: U },
	Remove { size: T },
	Replace { replace_size: T, size: T, data: U },
	ReplaceWithSameLength { size: T, data: U },
}

impl<U: Read> IntoBytesSerializer for DiffBlock<u32, U> {
	type Item = DiffBlock<u32, U>;

	fn into_bytes(self) -> BytesSerializer<Self::Item> {
		return BytesSerializer::new(
			self,
			Box::new(
				|position: &mut usize, val, mut buffer: &mut [u8]| match val {
					DiffBlock::Skip { size } => {
						if *position < 6 {
							let mut bytes = &mut [0u8; 2 + 4][..];
							bytes[0..2].clone_from_slice(&u16_to_u8_be_vec(&0u16)[..]);
							bytes[2..6].clone_from_slice(&u32_to_u8_be_vec(&size)[..]);
							let res = Cursor::new(&bytes[*position..]).read(&mut buffer)?;
							*position += res;
							return Ok(res);
						} else {
							return Ok(0);
						}
					}
					DiffBlock::Add { size, ref mut data } => {
						if *position < 6 {
							let mut bytes = &mut [0u8; 2 + 4][..];
							bytes[0..2].clone_from_slice(&u16_to_u8_be_vec(&1u16)[..]);
							bytes[2..6].clone_from_slice(&u32_to_u8_be_vec(&size)[..]);
							let res = Cursor::new(&bytes[*position..])
								.chain(data)
								.read(&mut buffer)?;
							*position += res;
							return Ok(res);
						} else {
							return data.read(&mut buffer);
						}
					}
					DiffBlock::Remove { size } => {
						if *position < 6 {
							let mut bytes = &mut [0u8; 2 + 4][..];
							bytes[0..2].clone_from_slice(&u16_to_u8_be_vec(&2u16)[..]);
							bytes[2..6].clone_from_slice(&u32_to_u8_be_vec(&size)[..]);
							let res = Cursor::new(&bytes[*position..]).read(&mut buffer)?;
							*position += res;
							return Ok(res);
						} else {
							return Ok(0);
						}
					}
					DiffBlock::Replace {
						replace_size,
						size,
						ref mut data,
					} => {
						if *position < 10 {
							let mut bytes = &mut [0u8; 2 + 4 + 4][..];
							bytes[0..2].clone_from_slice(&u16_to_u8_be_vec(&3u16)[..]);
							bytes[2..6].clone_from_slice(&u32_to_u8_be_vec(&replace_size)[..]);
							bytes[6..10].clone_from_slice(&u32_to_u8_be_vec(&size)[..]);
							let res = Cursor::new(&bytes[*position..])
								.chain(data)
								.read(&mut buffer)?;
							*position += res;
							return Ok(res);
						} else {
							return data.read(&mut buffer);
						}
					}
					DiffBlock::ReplaceWithSameLength { size, ref mut data } => {
						if *position < 6 {
							let mut bytes = &mut [0u8; 2 + 4][..];
							bytes[0..2].clone_from_slice(&u16_to_u8_be_vec(&4u16)[..]);
							bytes[2..6].clone_from_slice(&u32_to_u8_be_vec(&size)[..]);
							let res = Cursor::new(&bytes[*position..])
								.chain(data)
								.read(&mut buffer)?;
							*position += res;
							return Ok(res);
						} else {
							return data.read(&mut buffer);
						}
					}
				},
			),
		);
	}
}

#[cfg(test)]
mod diff_block_tests {
	use super::super::bytes_serializer::IntoBytesSerializer;
	use super::DiffBlock;
	use std::io::{Cursor, Read};

	#[test]
	fn diffblock_read_test() {
		let data = Cursor::new([1, 2, 3, 4, 5, 6]);
		let block = DiffBlock::Add {
			size: 6,
			data: data,
		};
		let mut buf = vec![0; 2 + 4 + 6];
		block.into_bytes().read_exact(&mut buf).unwrap();
		assert_eq!(
			buf,
			[
				0x00, 0x01, // action
				0x00, 0x00, 0x00, 6, //size
				1, 2, 3, 4, 5, 6 // data
			]
		);
	}
}
