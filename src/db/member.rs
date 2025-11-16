use chrono_tz::Tz;
use miette::{miette, IntoDiagnostic, Report, Result};
use nanowrimo::{NanoKind, ProjectObject};
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

use crate::{
	bot::App,
	db::{project::Project, trackbear_login::TrackbearLogin},
};

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

	pub async fn timezone(self, app: App) -> Result<Option<Tz>> {
		let user = if let Some(login) = TrackbearLogin::get_for_member(app.clone(), self).await? {
			let client = login.client().await?;
			client.current_user().await.into_diagnostic()?
		} else if let Some(project) = Project::get_for_member(app.clone(), self).await? {
			let client = TrackbearLogin::client_for_member(app.clone(), self).await?;
			let nano_project = client
				.get_id::<ProjectObject>(NanoKind::Project, project.nano_id)
				.await
				.into_diagnostic()?;
			client
				.get_id(NanoKind::User, nano_project.data.attributes.user_id)
				.await
				.into_diagnostic()?
		} else {
			return Ok(None);
		};

		user.data
			.attributes
			.time_zone
			.parse()
			.map(Some)
			.map_err(|err| miette!("nano: fetch project user timezone: {}", err))
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
