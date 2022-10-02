use std::{ops::Deref, sync::Arc};

use miette::{IntoDiagnostic, Result};
use sqlx::PgPool;
use tokio::sync::mpsc::Sender;

use super::action::Action;
use crate::config::Config;

#[derive(Clone, Debug)]
#[repr(transparent)]
pub struct App(Arc<AppContext>);

#[derive(Clone, Debug)]
pub struct AppContext {
	pub config: Config,
	pub db: PgPool,
	pub control: Sender<Action>,
}

impl App {
	pub fn new(config: Config, db: PgPool, control: Sender<Action>) -> Self {
		Self(Arc::new(AppContext {
			config,
			db,
			control,
		}))
	}

	pub async fn send_action(&self, action: Action) -> Result<()> {
		self.control.send(action).await.into_diagnostic()
	}
}

impl Deref for App {
	type Target = AppContext;

	fn deref(&self) -> &Self::Target {
		&self.0
	}
}
