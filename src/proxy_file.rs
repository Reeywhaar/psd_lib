use std::convert::From;
use std::fs::{remove_file, rename, File};
use std::io::{stdout, BufWriter, Error, Result as IOResult, Stdout, Write};
use std::ops::Drop;
use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};

fn timestamp() -> String {
	let now = SystemTime::now();
	let elapsed = now
		.duration_since(UNIX_EPOCH)
		.expect("Error getting timestamp");
	elapsed.as_secs().to_string()
}

fn create_tempname(path: &str) -> String {
	format!("{}.tmp.{}", path, timestamp())
}

pub struct ProxyFile {
	original_path: String,
	temp_path: String,
	writer: Box<dyn Write>,
	err: Option<Error>,
}

impl ProxyFile {
	pub fn end(mut self) -> Result<(), String> {
		if self.original_path == "-" {
			(&mut self.writer).flush().map_err(|x| x.to_string())?;
		} else if self.err.is_some() {
			remove_file(&self.temp_path).map_err(|x| x.to_string())?;
		} else {
			(&mut self.writer).flush().map_err(|x| x.to_string())?;
			rename(&self.temp_path, &self.original_path).map_err(|x| x.to_string())?;
		};
		Ok(())
	}
}

impl Write for ProxyFile {
	fn write(&mut self, buffer: &[u8]) -> IOResult<usize> {
		if let Some(ref e) = self.err {
			return Err(Error::from(e.kind()));
		};
		let res = self.writer.write(buffer);
		if let Err(ref e) = res {
			self.err = Some(Error::from(e.kind()));
		};
		res
	}

	fn flush(&mut self) -> IOResult<()> {
		if let Some(ref e) = self.err {
			return Err(Error::from(e.kind()));
		};
		let res = self.writer.flush();
		if let Err(ref e) = res {
			self.err = Some(Error::from(e.kind()));
		};
		res
	}
}

impl From<String> for ProxyFile {
	fn from(v: String) -> Self {
		if v == "-" {
			return Self {
				original_path: "-".to_string(),
				temp_path: "-".to_string(),
				writer: Box::new(BufWriter::with_capacity(1024 * 64, stdout())),
				err: None,
			};
		};
		let tempname = create_tempname(&v);
		let file = File::create(&tempname).expect("Cannot create temporary file");
		Self {
			original_path: v,
			temp_path: tempname,
			writer: Box::new(BufWriter::with_capacity(1024 * 64, file)),
			err: None,
		}
	}
}

impl From<PathBuf> for ProxyFile {
	fn from(path: PathBuf) -> Self {
		let path = path.to_string_lossy().to_string();
		Self::from(path)
	}
}

impl From<Stdout> for ProxyFile {
	fn from(v: Stdout) -> Self {
		Self {
			original_path: "-".to_string(),
			temp_path: "-".to_string(),
			writer: Box::new(v),
			err: None,
		}
	}
}

impl Drop for ProxyFile {
	fn drop(&mut self) {
		if self.original_path != "-" {
			let path = PathBuf::from(&self.temp_path);
			if path.exists() {
				remove_file(&path).expect("Cannot remove temp file");
			};
		};
	}
}
