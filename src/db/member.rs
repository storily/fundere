use miette::{miette, IntoDiagnostic, Report, Result};
use postgres_types::{FromSql, ToSql};
use twilight_mention::{fmt::MentionFormat, Mention};
use twilight_model::{
	application::interaction::Interaction,
	guild::Member as DiscordMember,
	id::{
		marker::{GuildMarker, UserMarker},
		Id,
	},
};
use uuid::Uuid;

use crate::bot::App;

// Discord snowflake IDs will never (read: unless they either change the
// schema or we're 10k years in the future) reach even 60 bits of length
// so we're quite safe casting them to i64

#[derive(Debug, Clone, Copy, ToSql, FromSql)]
#[postgres(name = "member")]
pub struct Member {
	pub guild_id: i64,
	pub user_id: i64,
}

impl Member {
	pub async fn to_member(self, app: App) -> Result<DiscordMember> {
		app.client
			.guild_member(self.into(), self.into())
			.await
			.into_diagnostic()?
			.model()
			.await
			.into_diagnostic()
	}

	pub async fn name(self, app: App) -> Result<String> {
		let member = self.to_member(app).await?;
		Ok(member.nick.unwrap_or(member.user.name))
	}
}

impl Mention<Id<UserMarker>> for Member {
	fn mention(&self) -> MentionFormat<Id<UserMarker>> {
		Id::<UserMarker>::from(*self).mention()
	}
}

impl From<Member> for Id<UserMarker> {
	fn from(chan: Member) -> Self {
		Id::new(chan.user_id as _)
	}
}

impl From<Member> for Id<GuildMarker> {
	fn from(chan: Member) -> Self {
		Id::new(chan.guild_id as _)
	}
}

// Uuid<->Member to represent a member as a single number/string/field
impl From<Member> for Uuid {
	fn from(Member { guild_id, user_id }: Member) -> Self {
		// snowflakes are unsigned, only i64 here to store in postgres
		Self::from_u64_pair(guild_id as _, user_id as _)
	}
}
impl From<Uuid> for Member {
	fn from(id: Uuid) -> Self {
		let (guild, user) = id.as_u64_pair();
		Self {
			guild_id: guild as _,
			user_id: user as _,
		}
	}
}

impl TryFrom<&Interaction> for Member {
	type Error = Report;

	fn try_from(interaction: &Interaction) -> Result<Self> {
		let guild_id = interaction
			.guild_id
			.ok_or(miette!("interaction is not from a guild"))?;
		let user = interaction
			.member
			.as_ref()
			.and_then(|m| m.user.as_ref())
			.ok_or(miette!("interaction is not from a guild"))?;
		Ok(Self {
			guild_id: guild_id.get() as _,
			user_id: user.id.get() as _,
		})
	}
}
