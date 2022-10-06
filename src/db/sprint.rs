use std::{str::FromStr, time::Duration};

use chrono::{DateTime, TimeZone, Utc};
use miette::{Context, IntoDiagnostic, Result};
use sqlx::{
	postgres::{types::PgInterval, PgHasArrayType, PgTypeInfo},
	types::Uuid,
	Postgres, Row, Type, TypeInfo,
};
use strum::{Display, EnumString};
use twilight_mention::{fmt::MentionFormat, Mention};
use twilight_model::{
	guild::Member as DiscordMember,
	id::{
		marker::{GuildMarker, UserMarker},
		Id,
	},
	user::User,
};

use crate::bot::App;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, EnumString, Display)]
#[strum(serialize_all = "lowercase")]
pub enum SprintStatus {
	Initial,
	Announced,
	Started,
	Ended,
	Summaried,
}

#[derive(Debug, Clone, sqlx::Encode, sqlx::Decode)]
pub struct Member {
	pub guild_id: i64,
	pub user_id: i64,
}

impl Type<Postgres> for Member {
	fn type_info() -> PgTypeInfo {
		PgTypeInfo::with_name("member_t")
	}

	fn compatible(ty: &<Postgres as sqlx::Database>::TypeInfo) -> bool {
		ty.name() == "member" || ty.name() == "member_t"
	}
}

impl Member {
	pub async fn to_user(&self, app: App) -> Result<User> {
		app.client
			.user(Id::new(self.user_id as _))
			.exec()
			.await
			.into_diagnostic()?
			.model()
			.await
			.into_diagnostic()
	}

	pub async fn to_member(&self, app: App) -> Result<DiscordMember> {
		app.client
			.guild_member(Id::new(self.guild_id as _), Id::new(self.user_id as _))
			.exec()
			.await
			.into_diagnostic()?
			.model()
			.await
			.into_diagnostic()
	}
}

impl Mention<Id<UserMarker>> for Member {
	fn mention(&self) -> MentionFormat<Id<UserMarker>> {
		Id::new(self.user_id as _).mention()
	}
}

#[derive(Debug, Clone, sqlx::FromRow, sqlx::Encode, sqlx::Decode)]
pub struct Participant {
	pub sprint_id: Uuid,
	pub member: Member,
	pub joined_at: DateTime<Utc>,
	pub words_start: Option<i32>,
	pub words_end: Option<i32>,
}

impl Type<Postgres> for Participant {
	fn type_info() -> PgTypeInfo {
		PgTypeInfo::with_name("participant")
	}

	fn compatible(ty: &<Postgres as sqlx::Database>::TypeInfo) -> bool {
		ty.name() == "participant"
	}
}

impl Mention<Id<UserMarker>> for Participant {
	fn mention(&self) -> MentionFormat<Id<UserMarker>> {
		self.member.mention()
	}
}

impl PgHasArrayType for Participant {
	fn array_type_info() -> PgTypeInfo {
		PgTypeInfo::with_name("_sprint_participants")
	}
}

#[derive(Debug, Clone, sqlx::FromRow)]
pub struct Sprint {
	pub id: Uuid,
	pub shortid: i32,
	pub starting_at: DateTime<Utc>,
	pub duration: PgInterval,
	pub status: String,
	pub participants: Vec<Participant>,
}

impl Sprint {
	pub async fn create<TZ>(
		app: App,
		starting_at: DateTime<TZ>,
		duration: chrono::Duration,
	) -> Result<Uuid>
	where
		TZ: TimeZone,
		TZ::Offset: Send,
	{
		sqlx::query("INSERT INTO sprints (starting_at, duration) VALUES ($1, $2) RETURNING id")
			.bind(starting_at)
			.bind(duration)
			.fetch_one(&app.pool)
			.await
			.into_diagnostic()
			.wrap_err("storing to db")?
			.try_get("id")
			.into_diagnostic()
			.wrap_err("getting stored id")
	}

	pub async fn from_current(app: App, uuid: Uuid) -> Result<Self> {
		sqlx::query_as("SELECT * FROM sprints_current WHERE id = $1")
			.bind(uuid)
			.fetch_one(&app.pool)
			.await
			.into_diagnostic()
	}

	pub async fn update_status(&self, app: App, status: SprintStatus) -> Result<()> {
		sqlx::query("UPDATE sprints SET status = $2 WHERE id = $1")
			.bind(self.id)
			.bind(status.to_string())
			.execute(&app.pool)
			.await
			.into_diagnostic()
			.wrap_err("db: update sprint status")
			.map(drop)
	}

	pub async fn cancel(&self, app: App) -> Result<()> {
		sqlx::query("UPDATE sprints SET cancelled_at = CURRENT_TIMESTAMP WHERE id = $1")
			.bind(self.id)
			.execute(&app.pool)
			.await
			.into_diagnostic()
			.wrap_err("db: cancel sprint")
			.map(drop)
	}

	pub async fn join(
		&self,
		app: App,
		guild_id: Id<GuildMarker>,
		user_id: Id<UserMarker>,
	) -> Result<()> {
		// Discord snowflake IDs will never (read: unless they either change the
		// schema or we're 10k years in the future) reach even 60 bits of length
		// so we're quite safe casting them to i64

		sqlx::query("INSERT INTO sprint_participants (sprint_id, member.guild_id, member.user_id) VALUES ($1, $2, $3) ON CONFLICT DO NOTHING")
			.bind(self.id)
			.bind(guild_id.get() as i64)
			.bind(user_id.get() as i64)
			.execute(&app.pool)
			.await
			.into_diagnostic()
			.wrap_err("db: join sprint")
			.map(drop)
	}

	pub async fn leave(
		&self,
		app: App,
		guild_id: Id<GuildMarker>,
		user_id: Id<UserMarker>,
	) -> Result<()> {
		sqlx::query("DELETE FROM sprint_participants WHERE sprint_id = $1 AND (member).guild_id = $2 AND (member).user_id = $3")
			.bind(self.id)
			.bind(guild_id.get() as i64)
			.bind(user_id.get() as i64)
			.execute(&app.pool)
			.await
			.into_diagnostic()
			.wrap_err("db: leave sprint")
			.map(drop)
	}

	pub fn status(&self) -> Result<SprintStatus> {
		SprintStatus::from_str(&self.status).into_diagnostic()
	}

	pub fn duration(&self) -> Duration {
		Duration::from_secs(
			(self.duration.days as u64 + self.duration.months as u64 * 31)
				* 24 * 60 * 60 * 1_000_000
				+ (self.duration.microseconds as u64) / (1_000_000),
		)
	}

	pub fn starting_in(&self) -> Option<Duration> {
		let now = Utc::now();
		if self.starting_at > now {
			Some(Duration::from_secs(
				(self.starting_at - now).num_seconds() as _
			))
		} else {
			None
		}
	}
}
