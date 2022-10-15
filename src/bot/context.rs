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
	application::{component::Component, interaction::Interaction},
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
use crate::{config::Config, db::sprint::Sprint};

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
	pub async fn send_response(&self, response: GenericResponse) -> Result<Message> {
		let posted_response = if let Some(msg) = response.message {
			Some(match msg {
				MessageForm::Discord(msg) => msg,
				MessageForm::Db(msgid) => self
					.client
					.message(msgid.into(), msgid.into())
					.exec()
					.await
					.into_diagnostic()?
					.model()
					.await
					.into_diagnostic()?,
			})
		} else if let Some(token) = &response.token {
			debug!("check if response already sent");
			self.get_response_message(token).await?
		} else {
			None
		};

		match (posted_response, response.interaction, response.token) {
			(None, None, _) | (None, _, None) => {
				debug!("no response and no interaction, post to channel")
			}
			(Some(msg), _, _)
				if SystemTime::now()
					>= (UNIX_EPOCH
						+ Duration::from_secs(msg.timestamp.as_secs().max(0) as u64 + 15 * 60)) =>
			{
				debug!("response already sent, but too old, post to channel instead")
			}
			(Some(_), _, None) => debug!("got a response but no interaction, post to channel"),
			(Some(_), _, Some(token)) => {
				debug!("response already sent, post followup");
				return response
					.data
					.incept_followup(self.interaction_client().create_followup(&token))?
					.exec()
					.await
					.into_diagnostic()
					.wrap_err("followup exec")?
					.model()
					.await
					.into_diagnostic()
					.wrap_err("followup response");
			}
			(None, Some(id), Some(token)) => {
				debug!("response not sent, post response");
				self.interaction_client()
					.create_response(
						id,
						&token,
						&InteractionResponse {
							kind: InteractionResponseType::ChannelMessageWithSource,
							data: Some(response.data.as_response()),
						},
					)
					.exec()
					.await
					.into_diagnostic()
					.wrap_err("create response")?;

				return self
					.get_response_message(&token)
					.await?
					.ok_or_else(|| miette!("no response message"));
			}
		}

		if let Some(channel) = response.channel {
			response
				.data
				.incept_message(self.client.create_message(channel))?
				.exec()
				.await
				.into_diagnostic()
				.wrap_err("message exec")?
				.model()
				.await
				.into_diagnostic()
				.wrap_err("message response")
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
	pub channel: Option<Id<ChannelMarker>>,
	pub interaction: Option<Id<InteractionMarker>>,
	pub token: Option<String>,
	pub message: Option<MessageForm>,
	pub data: GenericResponseData,
}

impl GenericResponse {
	pub fn from_interaction(interaction: &Interaction, data: GenericResponseData) -> Self {
		Self {
			channel: interaction.channel_id,
			interaction: Some(interaction.id),
			token: Some(interaction.token.clone()),
			message: None,
			data,
		}
	}

	pub fn from_sprint(sprint: &Sprint, data: GenericResponseData) -> Self {
		Self {
			channel: sprint.announce.map(|msg| msg.into()),
			interaction: None,
			token: Some(sprint.interaction_token.clone()),
			message: sprint.announce.map(MessageForm::Db),
			data,
		}
	}
}

#[derive(Debug, Clone)]
pub enum MessageForm {
	Discord(Message),
	Db(crate::db::message::Message),
}

#[derive(Debug, Clone, Default)]
pub struct GenericResponseData {
	pub ephemeral: bool,
	pub content: Option<String>,
	pub embeds: Vec<Embed>,
	pub components: Vec<Component>,
	pub attachments: Vec<Attachment>,
}

impl GenericResponseData {
	fn incept_followup<'f>(
		&'f self,
		mut followup: CreateFollowup<'f>,
	) -> Result<CreateFollowup<'f>> {
		if self.ephemeral {
			followup = followup.flags(MessageFlags::EPHEMERAL);
		}
		if let Some(content) = &self.content {
			followup = followup
				.content(content)
				.into_diagnostic()
				.wrap_err("followup content")?;
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
		if let Some(content) = &self.content {
			message = message
				.content(content)
				.into_diagnostic()
				.wrap_err("message content")?;
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
		if let Some(content) = self.content {
			ic_response = ic_response.content(content);
		}
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
