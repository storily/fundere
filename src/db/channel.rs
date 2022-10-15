use std::error::Error;

use miette::{miette, IntoDiagnostic, Report, Result};
use postgres_types::{FromSql, Kind, ToSql, Type};
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
#[postgres(name = "channel_t")]
struct ChannelInner {
	pub guild_id: i64,
	pub channel_id: i64,
}

#[derive(Debug, Clone, Copy, ToSql)]
#[postgres(name = "channel")]
pub struct Channel(ChannelInner);

impl<'a> FromSql<'a> for Channel {
	// Working around a buggy FromSql derive which didn't unwrap the inner type of
	// the domain before passing it to ChannelInner.
	fn from_sql(
		outer_type: &Type,
		buf: &'a [u8],
	) -> std::result::Result<Channel, Box<dyn Error + Sync + Send>> {
		ChannelInner::from_sql(
			match outer_type.kind() {
				Kind::Domain(domain) => domain,
				_ => unreachable!("assumption: channel is a domain"),
			},
			buf,
		)
		.map(Self)
	}

	// This is just copy-pasted from the derive expansion.
	fn accepts(outer_type: &Type) -> bool {
		if <ChannelInner as FromSql>::accepts(outer_type) {
			return true;
		}

		if outer_type.name() != "channel" {
			return false;
		}

		match outer_type.kind() {
			Kind::Domain(inner_type) => <ChannelInner as FromSql>::accepts(inner_type),
			_ => false,
		}
	}
}

impl Channel {
	#[allow(dead_code)]
	pub async fn to_channel(&self, app: App) -> Result<DiscordChannel> {
		app.client
			.channel(Id::new(self.0.channel_id as _))
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
		Id::new(chan.0.channel_id as _)
	}
}

impl From<Channel> for Id<GuildMarker> {
	fn from(chan: Channel) -> Self {
		Id::new(chan.0.guild_id as _)
	}
}

impl TryFrom<&Interaction> for Channel {
	type Error = Report;

	fn try_from(interaction: &Interaction) -> Result<Self> {
		ChannelInner::try_from(interaction).map(Self)
	}
}

impl TryFrom<&Interaction> for ChannelInner {
	type Error = Report;

	fn try_from(interaction: &Interaction) -> Result<Self> {
		let guild_id = interaction
			.guild_id
			.ok_or(miette!("interaction is not from a guild"))?;
		let channel = interaction
			.channel_id
			.ok_or(miette!("interaction is not from a guild"))?;
		Ok(Self {
			guild_id: guild_id.get() as _,
			channel_id: channel.get() as _,
		})
	}
}

impl TryFrom<&DiscordMessage> for Channel {
	type Error = Report;

	fn try_from(msg: &DiscordMessage) -> Result<Self> {
		Ok(Self(ChannelInner {
			guild_id: msg
				.guild_id
				.ok_or_else(|| miette!("not a guild message!"))?
				.get() as _,
			channel_id: msg.channel_id.get() as _,
		}))
	}
}
