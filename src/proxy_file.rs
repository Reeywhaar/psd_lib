use std::convert::From;
use std::fs::{remove_file, rename, File};
use std::io::{stdout, BufWriter, Error, Result as IOResult, Stdout, Write};
use std::ops::Drop;
use std::time::{SystemTime, UNIX_EPOCH};

fn timestamp() -> String {
	let now = SystemTime::now();
	let elapsed = now
		.duration_since(UNIX_EPOCH)
		.expect("Error getting timestamp");
	return elapsed.as_secs().to_string();
}

fn create_tempname(path: &str) -> String {
	return format!("{}.tmp.{}", path, timestamp());
}

pub struct ProxyFile {
	original_path: String,
	temp_path: String,
	writer: Box<dyn Write>,
	err: Option<Error>,
}

impl ProxyFile {
	pub fn set_err(&mut self, err: Error) {
		self.err = Some(err);
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
		return res;
	}

	fn flush(&mut self) -> IOResult<()> {
		if let Some(ref e) = self.err {
			return Err(Error::from(e.kind()));
		};
		let res = self.writer.flush();
		if let Err(ref e) = res {
			self.err = Some(Error::from(e.kind()));
		};
		return res;
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
		if self.original_path == "-" {
			self.writer.flush().expect("Cannot flush stdout");
		} else {
			if self.err.is_some() {
				remove_file(&self.temp_path).expect("Cannot remove tempfile");
			} else {
				self.writer
					.flush()
					.expect("Cannot flush output to tempfile");
				rename(&self.temp_path, &self.original_path)
					.expect("Cannot move tempfile to destination");
			}
		}
	}
}
