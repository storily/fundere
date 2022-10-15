use std::{
	ops::Deref,
	sync::Arc,
	time::{Duration, Instant, SystemTime, UNIX_EPOCH},
};

use miette::{miette, Context, IntoDiagnostic, Result};
use tokio::{
	sync::mpsc::Sender,
	time::{sleep_until, timeout, Instant as TokioInstant, Sleep},
};
use tokio_postgres::Client as PgClient;
use tracing::debug;
use twilight_http::{
	client::InteractionClient,
	error::ErrorType,
	request::{application::interaction::CreateFollowup, channel::message::CreateMessage},
	Client,
};
use twilight_model::{
	application::component::Component,
	channel::{embed::Embed, message::MessageFlags, Message},
	http::{
		attachment::Attachment,
		interaction::{InteractionResponse, InteractionResponseData, InteractionResponseType},
	},
	id::{
		marker::{ChannelMarker, InteractionMarker},
		Id,
	},
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
		channel: Option<Id<ChannelMarker>>,
		interaction: Option<Id<InteractionMarker>>,
		token: &str,
		response: GenericResponse,
	) -> Result<()> {
		debug!("check if response already sent");
		let posted_response = self.get_response_message(token).await?;

		match (posted_response, interaction) {
			(None, None) => debug!("no response and no id, post to channel"),
			(Some(msg), _)
				if SystemTime::now()
					>= (UNIX_EPOCH
						+ Duration::from_secs(msg.timestamp.as_secs().max(0) as u64 + 15 * 60)) =>
			{
				debug!("response already sent, but too old, post to channel instead")
			}
			(Some(_), _) => {
				debug!("response already sent, post followup");
				return response
					.incept_followup(self.interaction_client().create_followup(token))?
					.exec()
					.await
					.into_diagnostic()
					.wrap_err("followup exec")?
					.model()
					.await
					.into_diagnostic()
					.wrap_err("followup response")
					.map(drop);
			}
			(None, Some(id)) => {
				debug!("response not sent, post response");
				return self
					.interaction_client()
					.create_response(
						id,
						token,
						&InteractionResponse {
							kind: InteractionResponseType::ChannelMessageWithSource,
							data: Some(response.as_response()),
						},
					)
					.exec()
					.await
					.into_diagnostic()
					.wrap_err("create response")
					.map(drop);
			}
		}

		if let Some(channel) = channel {
			response
				.incept_message(self.client.create_message(channel))?
				.exec()
				.await
				.into_diagnostic()
				.wrap_err("message exec")?
				.model()
				.await
				.into_diagnostic()
				.wrap_err("message response")
				.map(drop)
		} else {
			Err(miette!("cannot post response, possibly a bug?"))
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

impl GenericResponse {
	fn incept_followup<'f>(
		&'f self,
		mut followup: CreateFollowup<'f>,
	) -> Result<CreateFollowup<'f>> {
		if self.ephemeral {
			followup = followup.flags(MessageFlags::EPHEMERAL);
		}
		if !self.embeds.is_empty() {
			followup = followup
				.embeds(&self.embeds)
				.into_diagnostic()
				.wrap_err("followup embed")?;
		}
		if !self.components.is_empty() {
			followup = followup
				.components(&self.components)
				.into_diagnostic()
				.wrap_err("followup components")?;
		}
		if !self.attachments.is_empty() {
			followup = followup
				.attachments(&self.attachments)
				.into_diagnostic()
				.wrap_err("followup attachments")?;
		}
		Ok(followup)
	}

	fn incept_message<'f>(&'f self, mut message: CreateMessage<'f>) -> Result<CreateMessage<'f>> {
		if self.ephemeral {
			message = message.flags(MessageFlags::EPHEMERAL);
		}
		if !self.embeds.is_empty() {
			message = message
				.embeds(&self.embeds)
				.into_diagnostic()
				.wrap_err("message embed")?;
		}
		if !self.components.is_empty() {
			message = message
				.components(&self.components)
				.into_diagnostic()
				.wrap_err("message components")?;
		}
		if !self.attachments.is_empty() {
			message = message
				.attachments(&self.attachments)
				.into_diagnostic()
				.wrap_err("message attachments")?;
		}
		Ok(message)
	}

	fn as_response(self) -> InteractionResponseData {
		let mut ic_response = InteractionResponseDataBuilder::new();
		if self.ephemeral {
			ic_response = ic_response.flags(MessageFlags::EPHEMERAL);
		}
		if !self.embeds.is_empty() {
			ic_response = ic_response.embeds(self.embeds.into_iter());
		}
		if !self.components.is_empty() {
			ic_response = ic_response.components(self.components.into_iter());
		}
		if !self.attachments.is_empty() {
			ic_response = ic_response.attachments(self.attachments.into_iter());
		}
		ic_response.build()
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
