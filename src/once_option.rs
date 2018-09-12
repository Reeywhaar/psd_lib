use std::ops::Deref;

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

	pub fn or_default(self, def: T) -> T {
		if self.v.is_some() {
			return self.v.unwrap();
		} else {
			return def;
		};
	}
}

impl<T> AsRef<Option<T>> for OnceOption<T> {
	fn as_ref(&self) -> &Option<T> {
		return &self.v;
	}
}

impl<T> Deref for OnceOption<T> {
	type Target = Option<T>;

	fn deref(&self) -> &Self::Target {
		return &self.v;
	}
}
