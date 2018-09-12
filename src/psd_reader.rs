//! Contains `PSDReader` struct

use bin_diff::functions::{read_usize_be, u_to_i16_be};
use bin_diff::indexes::Indexes;
use std::collections::HashMap;
use std::io::{Read, Seek, SeekFrom};

static BPS_SIGNATURE: [u8; 4] = [0x38, 0x42, 0x50, 0x53];
static BIM_SIGNATURE: [u8; 4] = [0x38, 0x42, 0x49, 0x4D];
static B64_SIGNATURE: [u8; 4] = [0x38, 0x42, 0x36, 0x34];

enum PSDType {
	PSD,
	PSB,
}

impl PSDType {
	fn length(&self) -> u8 {
		match self {
			PSDType::PSD => 4,
			PSDType::PSB => 8,
		}
	}
}

/// PSDReader structure used to get `Indexes` from psd file
pub struct PSDReader<'a, T: 'a + Read + Seek> {
	file: &'a mut T,
	indexes: Option<Indexes>,
	pos: u64,
	starts: Box<HashMap<String, u64>>,
	ends: Box<HashMap<String, u64>>,
	order: Vec<String>,
	file_type: PSDType,
}

impl<'a, T: 'a + Read + Seek> PSDReader<'a, T> {
	pub fn new(file: &'a mut T) -> Self {
		return Self {
			file: file,
			indexes: None,
			pos: 0,
			starts: Box::new(HashMap::new()),
			ends: Box::new(HashMap::new()),
			order: vec![],
			file_type: PSDType::PSD,
		};
	}

	fn start(&mut self, label: &str) {
		// eprintln!("starting {:?} at {}", label, self.pos);
		self.starts.insert(label.to_string(), self.pos);
		self.order.push(label.to_string());
	}

	fn end(&mut self, label: &str) {
		// eprintln!("ending   {:?} at {}", label, self.pos);
		self.ends.insert(label.to_string(), self.pos);
	}

	fn advance(&mut self, label: &str, size: u64) {
		self.start(label);
		self.pos += size;
		self.end(label);
	}

	fn advance_and_read(&mut self, label: &str, size: u64) -> Result<u64, String> {
		self.start(label);

		self.file
			.seek(SeekFrom::Start(self.pos))
			.map_err(|err| err.to_string())?;
		let res = read_usize_be(&mut self.file, size as usize).map_err(|err| err.to_string())?;
		self.pos += size;
		self.end(label);

		return Ok(res as u64);
	}

	fn advance_and_read_vec(&mut self, label: &str, size: u64) -> Result<Vec<u8>, String> {
		self.start(label);

		self.file
			.seek(SeekFrom::Start(self.pos))
			.map_err(|err| err.to_string())?;
		let mut buf = vec![0; size as usize];
		self.file
			.read_exact(&mut buf)
			.map_err(|err| err.to_string())?;
		self.pos += size as u64;
		self.end(label);

		return Ok(buf);
	}

	fn advance_and_check(&mut self, label: &str, subj: &[u8]) -> Result<(), String> {
		let res = self.advance_and_read_vec(label, subj.len() as u64)?;
		if res != subj {
			return Err(format!("Check failed on \"{}\"", label));
		}

		return Ok(());
	}

	fn advance_and_check_multiple(&mut self, label: &str, subj: &[&[u8]]) -> Result<(), String> {
		let res = self.advance_and_read_vec(label, subj[0].len() as u64)?;
		for sub in subj {
			if res == *sub {
				return Ok(());
			}
		}

		return Err(format!("Check failed on \"{}\"", label));
	}

	fn pad(n: u64, pad: u64) -> u64 {
		let rem = n % pad;
		if rem == 0 {
			return n;
		}

		return n + (pad - rem);
	}

	fn get_header(&mut self) -> Result<(), String> {
		self.start("header");

		self.advance_and_check("header/signature", &BPS_SIGNATURE)?;

		let file_type = self.advance_and_read_vec("header/version", 2)?;
		match file_type.as_slice() {
			[0x00, 0x01] => self.file_type = PSDType::PSD,
			[0x00, 0x02] => self.file_type = PSDType::PSB,
			_ => return Err("Unknown File format".to_string()),
		}

		self.advance("header/reserved", 6);
		self.advance("header/number_of_channels", 2);
		self.advance("header/height", 4);
		self.advance("header/width", 4);
		self.advance("header/depth", 2);
		self.advance("header/color_mode", 2);

		self.end("header");
		return Ok(());
	}

	fn get_color_mode(&mut self) -> Result<(), String> {
		let len = self.advance_and_read("color_mode_section_length", 4)?;
		self.advance("color_mode_section", len);

		Ok(())
	}

	fn get_image_resource_section(&mut self) -> Result<(), String> {
		let len = self.advance_and_read("image_resources_length", 4)?;

		self.start("image_resources");

		let mut resource_index: u16 = 0;
		let end = self.pos + len as u64;

		while self.pos < end {
			self.file
				.seek(SeekFrom::Start(self.pos))
				.map_err(|x| x.to_string())?;
			self.start(&format!(
				"image_resources/image_resource_{}",
				resource_index
			));
			{
				let name_length;
				let data_length;

				self.advance_and_check_multiple(
					&format!(
						"image_resources/image_resource_{}/signature",
						resource_index
					),
					&[&BIM_SIGNATURE, &B64_SIGNATURE],
				)?;

				self.advance(
					&format!("image_resources/image_resource_{}/id", resource_index),
					2,
				);

				name_length = self.advance_and_read(
					&format!(
						"image_resources/image_resource_{}/name_length",
						resource_index
					),
					1,
				)?;

				if name_length == 0 {
					self.advance(
						&format!("image_resources/image_resource_{}/name", resource_index),
						1,
					);
				} else {
					self.advance(
						&format!("image_resources/image_resource_{}/name", resource_index),
						Self::pad(name_length + 1, 2) - 1,
					);
				}

				data_length = Self::pad(
					self.advance_and_read(
						&format!(
							"image_resources/image_resource_{}/data_length",
							resource_index
						),
						4,
					)?,
					2,
				);

				self.advance(
					&format!("image_resources/image_resource_{}/data", resource_index),
					data_length,
				);
			}
			self.end(&format!(
				"image_resources/image_resource_{}",
				resource_index
			));

			resource_index += 1;
		}

		self.end("image_resources");

		Ok(())
	}

	fn get_layer(&mut self, prefix: &String) -> Result<(), String> {
		let len = self.file_type.length() as u64;
		self.start(&prefix);

		self.start(&format!("{}/rect", prefix));
		self.advance(&format!("{}/rect/top", prefix), 4);
		self.advance(&format!("{}/rect/left", prefix), 4);
		self.advance(&format!("{}/rect/bottom", prefix), 4);
		self.advance(&format!("{}/rect/right", prefix), 4);
		self.end(&format!("{}/rect", prefix));

		self.start(&format!("{}/channel_info", prefix));

		let number_of_channels =
			self.advance_and_read(&format!("{}/channel_info:number", prefix), 2)?;

		{
			for i in 0..number_of_channels {
				self.start(&format!("{}/channel_info/channel_{}", prefix, i));
				self.advance(&format!("{}/channel_info/channel_{}/id", prefix, i), 2);
				self.advance(
					&format!("{}/channel_info/channel_{}:length", prefix, i),
					len,
				);
				self.end(&format!("{}/channel_info/channel_{}", prefix, i));
			}
		}
		self.end(&format!("{}/channel_info", prefix));

		self.advance_and_check_multiple(
			&format!("{}/blend_mode_signature", prefix),
			&[&BIM_SIGNATURE, &B64_SIGNATURE],
		)?;
		self.advance(&format!("{}/blend_mode_key", prefix), 4);
		self.advance(&format!("{}/opacity", prefix), 1);
		self.advance(&format!("{}/clipping", prefix), 1);
		self.advance(&format!("{}/flags", prefix), 1);
		self.advance(&format!("{}/filler", prefix), 1);

		let extra_data_length = self.advance_and_read(&format!("{}/extra_data_length", prefix), 4)?;

		let extra_data_end = self.pos + extra_data_length;

		self.start(&format!("{}/extra_data", prefix));
		{
			let mask_data_length =
				self.advance_and_read(&format!("{}/mask_data_length", prefix), 4)?;
			self.start(&format!("{}/mask_data", prefix));
			{
				if mask_data_length > 0 {
					self.start(&format!("{}/mask_data/rect", prefix));
					self.advance(&format!("{}/mask_data/rect/top", prefix), 4);
					self.advance(&format!("{}/mask_data/rect/left", prefix), 4);
					self.advance(&format!("{}/mask_data/rect/bottom", prefix), 4);
					self.advance(&format!("{}/mask_data/rect/right", prefix), 4);
					self.end(&format!("{}/mask_data/rect", prefix));

					self.advance(&format!("{}/mask_data/default_color", prefix), 1);

					let mask_flags =
						self.advance_and_read(&format!("{}/mask_data/flags", prefix), 1)?;

					if mask_flags & 0b00010000 != 0 {
						let params =
							self.advance_and_read(&format!("{}/mask_data/parameters", prefix), 1)?;
						if params & 0b10000000 != 0 {
							self.advance(&format!("{}/mask_data/user_mask_density", prefix), 1);
						}
						if params & 0b01000000 != 0 {
							self.advance(&format!("{}/mask_data/user_mask_feather", prefix), 2);
						}
						if params & 0b00100000 != 0 {
							self.advance(&format!("{}/mask_data/vector_mask_density", prefix), 1);
						}
						if params & 0b00010000 != 0 {
							self.advance(&format!("{}/mask_data/vector_mask_feather", prefix), 2);
						}
					}

					if mask_data_length == 20 {
						self.advance(&format!("{}/mask_data/padding", prefix), 2);
					} else {
						self.advance(&format!("{}/mask_data/real_flags", prefix), 1);

						self.advance(
							&format!("{}/mask_data/real_user_mask_background", prefix),
							1,
						);

						self.advance(&format!("{}/mask_data/real_rect", prefix), 16);
					}
				}
			}
			self.end(&format!("{}/mask_data", prefix));

			let blending_ranges_length =
				self.advance_and_read(&format!("{}/blending_ranges_length", prefix), 4)?;
			self.advance(
				&format!("{}/blending_ranges", prefix),
				blending_ranges_length,
			);

			let mut layer_name_length =
				self.advance_and_read(&format!("{}/name_length", prefix), 1)?;
			if layer_name_length > 1 {
				layer_name_length = Self::pad(layer_name_length + 1, 4) - 1;
			}
			self.advance(&format!("{}/name", prefix), layer_name_length);

			self.start(&format!("{}/additional_data", prefix));
			self.pos = extra_data_end;
			self.end(&format!("{}/additional_data", prefix));
		}
		self.end(&format!("{}/extra_data", prefix));
		self.end(prefix);

		return Ok(());
	}

	fn get_layers_resources(&mut self) -> Result<(), String> {
		let len = self.file_type.length() as u64;
		let layers_length = self.advance_and_read("layers_resources_length", len)?;
		let layers_end = self.pos + layers_length;

		self.start("layers_resources");
		{
			let layers_info_len =
				self.advance_and_read("layers_resources/layers_info_length", len)?;
			let layers_info_end = self.pos + layers_info_len;

			self.start("layers_resources/layers_info");
			{
				let layers_count =
					self.advance_and_read("layers_resources/layers_info/layer_count", 2)?;
				let mut layers_count = u_to_i16_be(layers_count as u16);
				// let channel_exists = layers_count < 0;
				if layers_count < 0 {
					layers_count *= -1;
				}

				let mut layer_index = 0;
				while layer_index < layers_count {
					self.get_layer(&format!(
						"layers_resources/layers_info/layer_{}",
						layer_index,
					))?;
					layer_index += 1;
				}

				self.start("layers_resources/layers_info/channel_data");
				{
					for i in 0..layers_count {
						self.start(&format!(
							"layers_resources/layers_info/channel_data/layer_{}",
							i
						));
						for j in 0.. {
							let len_bound = {
								let start = self.starts.get(
									&format!("layers_resources/layers_info/layer_{}/channel_info/channel_{}:length", i, j)
								);
								if start.is_none() {
									break;
								};
								let end = self.ends.get(
									&format!("layers_resources/layers_info/layer_{}/channel_info/channel_{}:length", i, j)
								);
								(start.unwrap().clone(), end.unwrap().clone())
							};
							{
								let len_len = len_bound.1 - len_bound.0;
								let init_pos = self.pos;
								let _ = self.file.seek(SeekFrom::Start(len_bound.0));
								let len = read_usize_be(&mut self.file, len_len as usize)
									.map_err(|x| x.to_string())?;
								let _ = self.file.seek(SeekFrom::Start(init_pos));
								self.pos = init_pos;
								self.start(&format!(
									"layers_resources/layers_info/channel_data/layer_{}/channel_{}",
									i, j
								));
								self.advance(&format!("layers_resources/layers_info/channel_data/layer_{}/channel_{}:compression_method", i, j), 2);
								self.advance(&format!("layers_resources/layers_info/channel_data/layer_{}/channel_{}:data", i, j), (len - 2) as u64);
								self.end(&format!(
									"layers_resources/layers_info/channel_data/layer_{}/channel_{}",
									i, j
								));
							}
						}
						self.end(&format!(
							"layers_resources/layers_info/channel_data/layer_{}",
							i
						));
					}

					if self.pos <= layers_info_end {
						let diff = layers_info_end - self.pos;
						self.advance("layers_resources/padding", diff);
					}
				}
				self.end("layers_resources/layers_info/channel_data");
				self.pos = layers_info_end;
			}
			self.end("layers_resources/layers_info");

			let global_mask_len = self.advance_and_read("layers_resources/global_mask_length", 4)?;
			self.advance("layers_resources/global_mask", global_mask_len);

			self.start("layers_resources/additional_layer_information");
			self.pos = layers_end;
			self.end("layers_resources/additional_layer_information");
		}
		self.end("layers_resources");

		Ok(())
	}

	fn get_image_data(&mut self) -> Result<(), String> {
		self.start("image_data");
		self.advance("image_data/compression_method", 2);
		self.start("image_data/data");
		let res = self
			.file
			.seek(SeekFrom::End(0))
			.map_err(|err| err.to_string())?;
		self.pos = res;
		self.end("image_data/data");
		self.end("image_data");

		Ok(())
	}

	/// Gets `Indexes`
	pub fn get_indexes(&mut self) -> Result<&Indexes, String> {
		if self.indexes.is_some() {
			return Ok(self.indexes.as_ref().unwrap());
		};

		let pos = self
			.file
			.seek(SeekFrom::Current(0))
			.map_err(|x| x.to_string())?;

		self.get_header()?;
		self.get_color_mode()?;
		self.get_image_resource_section()?;
		self.get_layers_resources()?;
		self.get_image_data()?;

		let mut indexes: Indexes = Indexes::new();

		for key in &self.order {
			let s = self
				.starts
				.get(key)
				.expect(&format!("failed to get key: {}", key))
				.clone();
			let e = self
				.ends
				.get(key)
				.expect(&format!("failed to get key: {}", key))
				.clone();
			if e < s {
				return Err(format!("end {} is shorter that start {} at {}", e, s, key));
			};
			indexes.insert(key.clone(), s, e - s);
		}

		self.starts.clear();
		self.ends.clear();

		self.indexes = Some(indexes);
		self.file
			.seek(SeekFrom::Start(pos))
			.map_err(|x| x.to_string())?;
		return Ok(self.indexes.as_ref().unwrap());
	}
}

#[cfg(test)]
mod psd_reader_tests {
	use super::PSDReader;
	use std::fs::File;

	#[test]
	fn get_indexes_test() {
		let file = File::open("./test_data/a_a.psd");
		let mut file = file.unwrap();
		let mut reader = PSDReader::new(&mut file);
		let r = reader.get_indexes().unwrap();
		assert!(r.has("header"));
		assert!(r.has("color_mode_section"));
		assert!(r.has("image_resources"));
		assert!(r.has("layers_resources"));
		assert!(r.has("image_data"));

		assert_eq!(r.get("header").unwrap(), (0, 26));
		assert_eq!(r.get("color_mode_section_length").unwrap(), (26, 4));
		assert_eq!(r.get("color_mode_section").unwrap(), (30, 0));
		assert_eq!(r.get("image_resources_length").unwrap(), (30, 4));
		assert_eq!(r.get("image_resources").unwrap(), (34, 61954));
		assert_eq!(r.get("layers_resources_length").unwrap(), (61988, 4));
		assert_eq!(r.get("layers_resources").unwrap(), (61992, 2072));
		assert_eq!(
			r.get("layers_resources/layers_info/channel_data").unwrap(),
			(62724, 1284)
		);
		assert_eq!(r.get("image_data").unwrap(), (64064, 1404));

		assert!(r.has("layers_resources/layers_info/layer_1"));
		assert!(!r.has("layers_resources/layers_info/layer_2"));
	}
}
