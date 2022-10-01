use std::{ops::Deref, sync::Arc};
use sqlx::{PgPool};

use crate::config::Config;

#[derive(Clone, Debug)]
#[repr(transparent)]
pub struct App(Arc<AppContext>);

#[derive(Clone, Debug)]
pub struct AppContext {
	pub config: Config,
	pub db: PgPool,
}

impl App {
	pub fn new(config: Config, db: PgPool) -> Self {
		Self(Arc::new(AppContext { config, db }))
	}
}

impl Deref for App {
	type Target = AppContext;

	fn deref(&self) -> &Self::Target {
		&self.0
	}
}
