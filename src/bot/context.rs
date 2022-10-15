use std::{
	ops::Deref,
	sync::Arc,
	time::{Duration, Instant},
};

use miette::{miette, Context, IntoDiagnostic, Result};
use tokio::{
	sync::mpsc::Sender,
	time::{sleep_until, timeout, Instant as TokioInstant, Sleep},
};
use tokio_postgres::Client as PgClient;
use tracing::debug;
use twilight_http::{client::InteractionClient, error::ErrorType, Client};
use twilight_model::{
	application::component::Component,
	channel::{embed::Embed, message::MessageFlags, Message},
	http::{
		attachment::Attachment,
		interaction::{InteractionResponse, InteractionResponseType},
	},
	id::{marker::InteractionMarker, Id},
};
use twilight_util::builder::InteractionResponseDataBuilder;

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
	pub timer: Sender<Timer>,
}

impl App {
	pub fn new(config: Config, db: PgClient, timer: Sender<Timer>) -> Self {
		let client = Client::new(config.discord.token.clone());
		Self(Arc::new(AppContext {
			config,
			db,
			client,
			timer,
		}))
	}

	pub fn interaction_client(&self) -> InteractionClient<'_> {
		let application_id = Id::new(self.config.discord.app_id);
		self.client.interaction(application_id)
	}

	pub async fn do_action(&self, action: Action) -> Result<()> {
		let action_dbg = format!("action: {action:?}");
		action.handle(self.clone()).await.wrap_err(action_dbg)
	}

	pub async fn send_timer(&self, timing: Timer) -> Result<()> {
		self.timer.send(timing).await.into_diagnostic()
	}

	#[tracing::instrument]
	pub async fn send_response(
		&self,
		id: Option<Id<InteractionMarker>>,
		token: &str,
		response: GenericResponse,
	) -> Result<()> {
		debug!("check if response already sent");
		let posted_response = self.get_response_message(token).await?;

		let ic = self.interaction_client();
		if posted_response.is_some() {
			debug!("build followup");
			let mut followup = ic.create_followup(token);
			if response.ephemeral {
				followup = followup.flags(MessageFlags::EPHEMERAL);
			}
			if !response.embeds.is_empty() {
				followup = followup
					.embeds(&response.embeds)
					.into_diagnostic()
					.wrap_err("followup embed")?;
			}
			if !response.components.is_empty() {
				followup = followup
					.components(&response.components)
					.into_diagnostic()
					.wrap_err("followup components")?;
			}
			if !response.attachments.is_empty() {
				followup = followup
					.attachments(&response.attachments)
					.into_diagnostic()
					.wrap_err("followup attachments")?;
			}

			debug!("send followup");
			followup
				.exec()
				.await
				.into_diagnostic()
				.wrap_err("followup exec")?
				.model()
				.await
				.into_diagnostic()
				.wrap_err("followup response")
				.map(drop)
		} else if let Some(id) = id {
			debug!("build response");
			let mut ic_response = InteractionResponseDataBuilder::new();
			if response.ephemeral {
				ic_response = ic_response.flags(MessageFlags::EPHEMERAL);
			}
			if !response.embeds.is_empty() {
				ic_response = ic_response.embeds(response.embeds.into_iter());
			}
			if !response.components.is_empty() {
				ic_response = ic_response.components(response.components.into_iter());
			}
			if !response.attachments.is_empty() {
				ic_response = ic_response.attachments(response.attachments.into_iter());
			}

			debug!("send response");
			ic.create_response(
				id,
				token,
				&InteractionResponse {
					kind: InteractionResponseType::ChannelMessageWithSource,
					data: Some(ic_response.build()),
				},
			)
			.exec()
			.await
			.into_diagnostic()
			.wrap_err("create response")
			.map(drop)
		} else {
			todo!()
		}
	}

	async fn get_response_message(&self, token: &str) -> Result<Option<Message>> {
		let ic = self.interaction_client();
		match timeout(
			Duration::from_millis(self.config.internal.response_lookup_timeout),
			ic.response(token).exec(),
		)
		.await
		{
			Ok(Ok(resp)) => resp
				.model()
				.await
				.into_diagnostic()
				.wrap_err("decode into Message")
				.map(Some),
			Ok(Err(err)) => match err.kind() {
				ErrorType::Response { status, .. } if status.get() == 404 => Ok(None),
				_ => Err(err).into_diagnostic().wrap_err("get response"),
			},
			Err(_) => Ok(None),
		}
	}
}

impl Deref for App {
	type Target = AppContext;

	fn deref(&self) -> &Self::Target {
		&self.0
	}
}

#[derive(Debug, Clone, Default)]
pub struct GenericResponse {
	pub ephemeral: bool,
	pub content: Option<String>,
	pub embeds: Vec<Embed>,
	pub components: Vec<Component>,
	pub attachments: Vec<Attachment>,
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
