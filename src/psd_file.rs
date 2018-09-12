//! Contains `PSDFile` struct

use bin_diff::indexes::{Indexes, WithIndexes};
use psd_reader::PSDReader;
use std::io::{Read, Result as IOResult, Seek, SeekFrom};

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

/// PSDFile implements `WithIndexes` trait from `bin_diff` package
pub struct PSDFile<T: Read + Seek> {
	file: T,
}

impl<T: Read + Seek> PSDFile<T> {
	pub fn new(file: T) -> Self {
		return Self { file: file };
	}

	fn get_indexes(&mut self) -> Result<Indexes, String> {
		let mut reader = PSDReader::new(&mut self.file);
		return reader.get_indexes().map(|x| x.clone());
	}

	pub fn get_lines(&mut self) -> Result<Indexes, String> {
		let mut out: Indexes = Indexes::new();
		let indexes = self.get_indexes()?;

		let findline = |line: String, out: &mut Indexes| -> Result<(), String> {
			let val = indexes
				.get(&line)
				.ok_or(format!("line \"{}\" wasn't found", line))?;
			out.insert(line, val.0, val.1);

			Ok(())
		};

		for line in LINES.iter() {
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

		return Ok(out);
	}
}

impl<T: Read + Seek> Read for PSDFile<T> {
	fn read(&mut self, mut buffer: &mut [u8]) -> IOResult<usize> {
		return self.file.read(&mut buffer);
	}
}

impl<T: Read + Seek> Seek for PSDFile<T> {
	fn seek(&mut self, from: SeekFrom) -> IOResult<u64> {
		return self.file.seek(from);
	}
}

impl<T: Read + Seek> WithIndexes for PSDFile<T> {
	fn get_indexes(&mut self) -> Result<Indexes, String> {
		return self.get_lines();
	}
}
