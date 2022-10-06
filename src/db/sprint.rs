use std::time::Duration;

use chrono::{DateTime, TimeZone, Utc};
use miette::{miette, Context, IntoDiagnostic, Result};
use pg_interval::Interval;
use postgres_types::{FromSql, ToSql};
use tokio_postgres::Row;
use twilight_mention::{fmt::MentionFormat, Mention};
use twilight_model::id::{
	marker::{GuildMarker, UserMarker},
	Id,
};
use uuid::Uuid;

use crate::bot::App;

use super::types::Member;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, ToSql, FromSql)]
pub enum SprintStatus {
	Initial,
	Announced,
	Started,
	Ended,
	Summaried,
}

#[derive(Debug, Clone, ToSql, FromSql)]
pub struct Participant {
	pub sprint_id: Uuid,
	pub member: Member,
	pub joined_at: DateTime<Utc>,
	pub words_start: Option<i32>,
	pub words_end: Option<i32>,
}

impl Mention<Id<UserMarker>> for Participant {
	fn mention(&self) -> MentionFormat<Id<UserMarker>> {
		self.member.mention()
	}
}

#[derive(Debug, Clone)]
pub struct Sprint {
	pub id: Uuid,
	pub shortid: i32,
	pub starting_at: DateTime<Utc>,
	pub duration: Interval,
	pub status: SprintStatus,
	pub participants: Vec<Participant>,
}

impl Sprint {
	fn from_row(row: Row) -> Result<Self> {
		Ok(Self {
			id: row.try_get("id").into_diagnostic()?,
			shortid: row.try_get("shortid").into_diagnostic()?,
			starting_at: row.try_get("starting_at").into_diagnostic()?,
			duration: row.try_get("duration").into_diagnostic()?,
			status: row.try_get("status").into_diagnostic()?,
			participants: match row.try_get("participants") {
				Ok(p) => p,
				Err(e) => match e {
					_ => todo!(),
				},
			},
		})
	}

	pub async fn create<TZ>(
		app: App,
		starting_at: DateTime<TZ>,
		duration: chrono::Duration,
	) -> Result<Uuid>
	where
		TZ: TimeZone,
	{
		app.db
			.query_one(
				"INSERT INTO sprints (starting_at, duration) VALUES ($1, $2) RETURNING id",
				&[
					&starting_at.with_timezone(&Utc),
					&Interval::from_duration(duration)
						.ok_or(miette!("could not convert duration to interval"))?,
				],
			)
			.await
			.and_then(|row| row.try_get("id"))
			.into_diagnostic()
			.wrap_err("db: create sprint")
	}

	pub async fn from_current(app: App, uuid: Uuid) -> Result<Self> {
		app.db
			.query_one("SELECT * FROM sprints_current WHERE id = $1", &[&uuid])
			.await
			.into_diagnostic()
			.and_then(Self::from_row)
			.wrap_err("db: get current sprint")
	}

	pub async fn update_status(&self, app: App, status: SprintStatus) -> Result<()> {
		app.db
			.query(
				"UPDATE sprints SET status = $2 WHERE id = $1",
				&[&self.id, &status],
			)
			.await
			.into_diagnostic()
			.wrap_err("db: update sprint status")
			.map(drop)
	}

	pub async fn cancel(&self, app: App) -> Result<()> {
		app.db
			.query(
				"UPDATE sprints SET cancelled_at = CURRENT_TIMESTAMP WHERE id = $1",
				&[&self.id],
			)
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

		app.db
			.query(
				"INSERT INTO sprint_participants (sprint_id, member.guild_id, member.user_id) VALUES ($1, $2, $3) ON CONFLICT DO NOTHING",
				&[&self.id, &(guild_id.get() as i64), &(user_id.get() as i64)],
			)
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
		app.db
			.query(
				"DELETE FROM sprint_participants WHERE sprint_id = $1 AND (member).guild_id = $2 AND (member).user_id = $3",
				&[&self.id, &(guild_id.get() as i64), &(user_id.get() as i64)],
			)
			.await
			.into_diagnostic()
			.wrap_err("db: leave sprint")
			.map(drop)
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
