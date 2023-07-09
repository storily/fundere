use miette::Report;
use tracing::error;

pub trait ErrorExt {
	fn log(self) -> Self;
	fn debug_log(self) -> Self;
}

impl<T> ErrorExt for Result<T, Report> {
	fn log(self) -> Self {
		if let Err(err) = &self {
			error!("{err}");
		}

		self
	}

	fn debug_log(self) -> Self {
		if let Err(err) = &self {
			error!("{err:?}");
		}

		self
	}
}
