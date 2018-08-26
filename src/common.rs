use consts::LINES;
use psd_reader::PSDReader;
use std::io::{Read, Seek};

pub fn get_lines<T: Read + Seek>(mut file: &mut T) -> Result<Vec<(String, u64, u64)>, String> {
	let indexes = {
		let mut r = PSDReader::new(&mut file);
		let indexes = r.get_indexes()?;
		indexes.clone()
	};

	let mut out: Vec<(String, u64, u64)> = vec![];

	let findline = |line: String, out: &mut Vec<(String, u64, u64)>| -> Result<(), String> {
		let val = indexes
			.get(&line)
			.ok_or(format!("line \"{}\" wasn't found", line))?;
		out.push((line, val.0, val.1));

		Ok(())
	};

	for line in LINES {
		let line = line.to_string();
		if line == "image_resources/image_resource_{n}" {
			let mut i = 0;
			while let Ok(()) = findline(format!("image_resources/image_resource_{}", i), &mut out) {
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
