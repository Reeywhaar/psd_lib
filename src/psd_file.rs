//! Contains `PSDFile` struct

use bin_diff::functions::{u32_to_u8_be_vec, u64_to_u8_be_vec};
use bin_diff::indexes::{Indexes, WithIndexes};
use psd_reader::PSDReader;
use std::convert::From;
use std::fs::File;
use std::io::{copy, Read, Result as IOResult, Seek, SeekFrom, Write};
use std::path::Path;

const LINES: [&str; 15] = [
	"header",
	"color_mode_section_length",
	"color_mode_section",
	"image_resources_length",
	"image_resources/image_resource_{n}",
	"layers_resources_length",
	"layers_resources/layers_info_length",
	"layers_resources/layers_info/layer_count",
	"layers_resources/layers_info/layer_{n}",
	"layers_resources/layers_info/channel_data/layer_{n}/channel_{n}",
	"layers_resources/layers_info/padding",
	"layers_resources/global_mask_length",
	"layers_resources/global_mask",
	"layers_resources/additional_layer_information",
	"image_data",
];

#[derive(Clone, Copy)]
pub enum PSDFileType {
	PSD,
	PSB,
}

/// PSDFile implements `WithIndexes` trait from `bin_diff` package
pub struct PSDFile<T: Read + Seek> {
	file: T,
	indexes: Option<Indexes>,
}

impl<T: Read + Seek> PSDFile<T> {
	pub fn new(file: T) -> Self {
		Self {
			file,
			indexes: None,
		}
	}

	pub fn get_indexes(&mut self) -> Result<&Indexes, String> {
		if self.indexes.is_none() {
			let mut reader = PSDReader::new(&mut self.file);
			self.indexes = Some(reader.get_indexes()?.clone());
		}
		Ok(self.indexes.as_ref().unwrap())
	}

	pub fn get_lines(&mut self) -> Result<Indexes, String> {
		let mut out: Indexes = Indexes::new();
		let indexes = self.get_indexes()?;

		let findline = |line: String, out: &mut Indexes| -> Result<(), String> {
			let val = indexes
				.get(&line)
				.ok_or_else(|| format!("line \"{}\" wasn't found", line))?;
			out.insert(line, val.0, val.1);

			Ok(())
		};

		for line in &LINES {
			let line = line.to_string();
			if line == "image_resources/image_resource_{n}" {
				let mut i = 0;
				while let Ok(()) =
					findline(format!("image_resources/image_resource_{}", i), &mut out)
				{
					i += 1;
				}
				continue;
			}
			if line == "layers_resources/layers_info/layer_{n}" {
				let mut i = 0;
				while let Ok(()) = findline(
					format!("layers_resources/layers_info/layer_{}", i),
					&mut out,
				) {
					i += 1;
				}
				continue;
			}
			if line == "layers_resources/layers_info/channel_data/layer_{n}/channel_{n}" {
				let mut layer = 0;
				loop {
					let mut channel = 0;
					while let Ok(()) = findline(
						format!(
							"layers_resources/layers_info/channel_data/layer_{}/channel_{}",
							layer, channel
						),
						&mut out,
					) {
						channel += 1;
					}
					if channel == 0 {
						break;
					}
					layer += 1;
				}
				continue;
			}
			findline(line, &mut out)?;
		}

		Ok(out)
	}

	/// writes composite (merged) psd file
	pub fn write_composite<W: Write>(&mut self, output: &mut W) -> Result<(), String> {
		let indexes = self.get_indexes()?.clone();
		let psd_type = {
			self.seek(SeekFrom::Start(4)).map_err(|x| x.to_string())?;
			let mut buf = [0; 2];
			self.read_exact(&mut buf).map_err(|x| x.to_string())?;
			match buf {
				[0, 1] => PSDFileType::PSD,
				[0, 2] => PSDFileType::PSB,
				_ => {
					return Err("Unknown PSD type".to_string());
				}
			}
		};
		let write_chunk = |label: &str, s: &mut PSDFile<T>, output: &mut W| -> Result<(), String> {
			let chunk = indexes
				.get(label)
				.ok_or_else(|| "cannot get label".to_string())?;
			s.seek(SeekFrom::Start(chunk.0))
				.map_err(|x| x.to_string())?;
			let mut taken = Read::by_ref(s).take(chunk.1);
			copy(&mut taken, output).map_err(|x| x.to_string())?;
			Ok(())
		};
		let layers_length = vec![
			"layers_resources/layers_info_length",
			"layers_resources/global_mask_length",
			"layers_resources/global_mask",
			"layers_resources/additional_layer_information",
		].iter()
		.map(|x| indexes.get(x).unwrap().1)
		.sum();

		write_chunk("header", self, output)?;
		write_chunk("color_mode_section_length", self, output)?;
		write_chunk("color_mode_section", self, output)?;
		output.write(&[0, 0, 0, 0]).map_err(|x| x.to_string())?; // image_resources_length
		match psd_type {
			PSDFileType::PSD => {
				output
					.write(&u32_to_u8_be_vec(layers_length as u32))
					.map_err(|x| x.to_string())?; // layers_resources_length
				output.write(&[0, 0, 0, 0]).map_err(|x| x.to_string())?; // layers_resources/layers_info_length
			}
			PSDFileType::PSB => {
				output
					.write(&u64_to_u8_be_vec(layers_length))
					.map_err(|x| x.to_string())?; // layers_resources_length
				output
					.write(&[0, 0, 0, 0, 0, 0, 0, 0])
					.map_err(|x| x.to_string())?; // layers_resources/layers_info_length
			}
		};
		write_chunk("layers_resources/global_mask_length", self, output)?;
		write_chunk("layers_resources/global_mask", self, output)?;
		write_chunk(
			"layers_resources/additional_layer_information",
			self,
			output,
		)?;
		write_chunk("image_data", self, output)?;
		Ok(())
	}
}

impl<T: AsRef<Path>> From<T> for PSDFile<File> {
	fn from(path: T) -> Self {
		let file = File::open(path).unwrap();
		Self {
			file,
			indexes: None,
		}
	}
}

impl<T: Read + Seek> Read for PSDFile<T> {
	fn read(&mut self, mut buffer: &mut [u8]) -> IOResult<usize> {
		self.file.read(&mut buffer)
	}
}

impl<T: Read + Seek> Seek for PSDFile<T> {
	fn seek(&mut self, from: SeekFrom) -> IOResult<u64> {
		self.file.seek(from)
	}
}

impl<T: Read + Seek> WithIndexes for PSDFile<T> {
	fn get_indexes(&mut self) -> Result<Indexes, String> {
		self.get_lines()
	}
}
