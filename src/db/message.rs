use miette::{Report, Result};
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
#[postgres(name = "message_t")]
struct MessageInner {
	pub channel: Channel,
	pub message_id: i64,
}

#[derive(Debug, Clone, Copy, ToSql, FromSql)]
#[postgres(name = "message")]
pub struct Message(MessageInner);

impl Message {
	pub fn channel(self) -> Channel {
		self.0.channel
	}
}

impl From<Message> for Id<MessageMarker> {
	fn from(msg: Message) -> Self {
		Id::new(msg.0.message_id as _)
	}
}

impl From<Message> for Id<ChannelMarker> {
	fn from(msg: Message) -> Self {
		Self::from(msg.channel())
	}
}

impl From<Message> for Id<GuildMarker> {
	fn from(msg: Message) -> Self {
		Self::from(msg.channel())
	}
}

impl TryFrom<&DiscordMessage> for Message {
	type Error = Report;

	fn try_from(msg: &DiscordMessage) -> Result<Self> {
		Ok(Self(MessageInner {
			channel: msg.try_into()?,
			message_id: msg.id.get() as _,
		}))
	}
}
