use std::{
	ops::Deref,
	sync::Arc,
	time::{Duration, Instant},
};

use miette::{miette, IntoDiagnostic, Result};
use tokio::{
	sync::mpsc::Sender,
	time::{sleep_until, Instant as TokioInstant, Sleep},
};
use tokio_postgres::Client as PgClient;
use twilight_http::{client::InteractionClient, Client};
use twilight_model::id::Id;

use super::action::Action;
use crate::config::Config;

#[derive(Clone, Debug)]
#[repr(transparent)]
pub struct App(Arc<AppContext>);

#[derive(Debug)]
pub struct AppContext {
	pub config: Config,
	pub db: PgClient,
	pub client: Client,
	pub control: Sender<Action>,
	pub timer: Sender<Timer>,
}

impl App {
	pub fn new(
		config: Config,
		db: PgClient,
		control: Sender<Action>,
		timer: Sender<Timer>,
	) -> Self {
		let client = Client::new(config.discord.token.clone());
		Self(Arc::new(AppContext {
			config,
			db,
			client,
			control,
			timer,
		}))
	}

	pub fn interaction_client(&self) -> InteractionClient<'_> {
		let application_id = Id::new(self.config.discord.app_id);
		self.client.interaction(application_id)
	}

	pub async fn send_action(&self, action: Action) -> Result<()> {
		self.control.send(action).await.into_diagnostic()
	}

	pub async fn send_timer(&self, timing: Timer) -> Result<()> {
		self.timer.send(timing).await.into_diagnostic()
	}
}

impl Deref for App {
	type Target = AppContext;

	fn deref(&self) -> &Self::Target {
		&self.0
	}
}

#[derive(Clone, Debug)]
pub struct Timer {
	pub until: TokioInstant,
	pub payload: Action,
}

impl Timer {
	pub fn new_at(time: Instant, payload: Action) -> Self {
		Self {
			until: time.into(),
			payload,
		}
	}

	pub fn new_after(duration: Duration, payload: Action) -> Result<Self> {
		Instant::now()
			.checked_add(duration)
			.ok_or_else(|| miette!("cannot schedule that far into the future"))
			.map(|time| Self::new_at(time.into(), payload))
	}

	pub fn to_sleep(&self) -> Sleep {
		sleep_until(self.until)
	}
}
