use miette::{Context, Report, Result};
use postgres_types::{FromSql, ToSql};
use twilight_model::{
	channel::message::Message as DiscordMessage,
	id::{
		marker::{ChannelMarker, GuildMarker, MessageMarker},
		Id,
	},
};

use super::channel::Channel;

#[derive(Debug, Clone, Copy, ToSql, FromSql)]
#[postgres(name = "message")]
pub struct Message {
	pub channel: Channel,
	pub message_id: i64,
}

impl From<Message> for Id<MessageMarker> {
	fn from(msg: Message) -> Self {
		Id::new(msg.message_id as _)
	}
}

impl From<Message> for Id<ChannelMarker> {
	fn from(msg: Message) -> Self {
		Self::from(msg.channel)
	}
}

impl TryFrom<Message> for Id<GuildMarker> {
	type Error = Report;
	fn try_from(msg: Message) -> Result<Self> {
		Self::try_from(msg.channel)
	}
}

impl TryFrom<&DiscordMessage> for Message {
	type Error = Report;

	fn try_from(msg: &DiscordMessage) -> Result<Self> {
		Ok(Self {
			channel: msg.try_into().wrap_err("convert channel")?,
			message_id: msg.id.get() as _,
		})
	}
}
