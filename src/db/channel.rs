use miette::{miette, IntoDiagnostic, Report, Result};
use postgres_types::{FromSql, ToSql};
use twilight_mention::{fmt::MentionFormat, Mention};
use twilight_model::{
	application::interaction::Interaction,
	channel::message::Message as DiscordMessage,
	channel::Channel as DiscordChannel,
	id::{
		marker::{ChannelMarker, GuildMarker},
		Id,
	},
};

use crate::bot::App;

#[derive(Debug, Clone, Copy, ToSql, FromSql)]
#[postgres(name = "channel")]
pub struct Channel {
	pub guild_id: Option<i64>,
	pub channel_id: i64,
}

impl Channel {
	#[allow(dead_code)]
	pub async fn to_channel(self, app: App) -> Result<DiscordChannel> {
		app.client
			.channel(self.into())
			.exec()
			.await
			.into_diagnostic()?
			.model()
			.await
			.into_diagnostic()
	}
}

impl Mention<Id<ChannelMarker>> for Channel {
	fn mention(&self) -> MentionFormat<Id<ChannelMarker>> {
		Id::<ChannelMarker>::from(*self).mention()
	}
}

impl From<Channel> for Id<ChannelMarker> {
	fn from(chan: Channel) -> Self {
		Id::new(chan.channel_id as _)
	}
}

impl TryFrom<Channel> for Id<GuildMarker> {
	type Error = Report;

	fn try_from(chan: Channel) -> Result<Self> {
		chan.guild_id
			.map(|id| Id::new(id as _))
			.ok_or_else(|| miette!("channel is not a guild channel"))
	}
}

impl TryFrom<&Interaction> for Channel {
	type Error = Report;

	fn try_from(interaction: &Interaction) -> Result<Self> {
		let guild_id = interaction
			.guild_id
			.ok_or(miette!("interaction is not from a guild"))?;
		let channel = interaction
			.channel_id
			.ok_or(miette!("interaction is not from a guild"))?;
		Ok(Self {
			guild_id: Some(guild_id.get() as _),
			channel_id: channel.get() as _,
		})
	}
}

impl TryFrom<&DiscordMessage> for Channel {
	type Error = Report;

	fn try_from(msg: &DiscordMessage) -> Result<Self> {
		Ok(Self {
			guild_id: msg.guild_id.map(|id| id.get() as _),
			channel_id: msg.channel_id.get() as _,
		})
	}
}
