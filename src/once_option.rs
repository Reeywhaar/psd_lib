/// Option that could be set only one time, all other attempts it will return `Err`
pub struct OnceOption<T> {
	v: Option<T>,
}

impl<T> OnceOption<T> {
	pub fn new() -> Self {
		Self { v: None }
	}

	pub fn set(&mut self, v: T) -> Result<(), String> {
		if self.v.is_some() {
			return Err("Cannot set value because it's already set".to_string());
		};

		self.v = Some(v);

		Ok(())
	}

	#[allow(dead_code)]
	pub fn unwrap(self) -> T {
		return self.v.unwrap();
	}

	pub fn or_default(self, def: T) -> Option<T> {
		if self.v.is_some() {
			return self.v;
		} else {
			return Some(def);
		};
	}
}

impl<T> AsRef<Option<T>> for OnceOption<T> {
	fn as_ref(&self) -> &Option<T> {
		return &self.v;
	}
}
