use std::{str::FromStr, time::Duration};

use chrono::{DateTime, TimeZone, Utc};
use miette::{Context, IntoDiagnostic, Result};
use sqlx::{postgres::types::PgInterval, types::Uuid, Row};
use strum::{Display, EnumString};
use twilight_model::id::{
	marker::{GuildMarker, UserMarker},
	Id,
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

#[derive(Debug, Clone, sqlx::FromRow)]
pub struct Sprint {
	pub id: Uuid,
	pub shortid: i32,
	pub starting_at: DateTime<Utc>,
	pub duration: PgInterval,
	pub status: String,
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
			.fetch_one(&app.db)
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
			.fetch_one(&app.db)
			.await
			.into_diagnostic()
	}

	pub async fn update_status(&self, app: App, status: SprintStatus) -> Result<()> {
		sqlx::query("UPDATE sprints SET status = $2 WHERE id = $1")
			.bind(self.id)
			.bind(status.to_string())
			.execute(&app.db)
			.await
			.into_diagnostic()
			.wrap_err("db: update sprint status")
			.map(drop)
	}

	pub async fn cancel(&self, app: App) -> Result<()> {
		sqlx::query("UPDATE sprints SET cancelled_at = CURRENT_TIMESTAMP WHERE id = $1")
			.bind(self.id)
			.execute(&app.db)
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
			.execute(&app.db)
			.await
			.into_diagnostic()
			.wrap_err("db: join sprint")
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
