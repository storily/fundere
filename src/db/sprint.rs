use std::time::Duration;

use chrono::{DateTime, TimeZone, Utc};
use miette::{miette, Context, IntoDiagnostic, Result};
use pg_interval::Interval;
use postgres_types::{FromSql, ToSql};
use tokio_postgres::Row;
use twilight_mention::{fmt::MentionFormat, Mention};
use twilight_model::id::{marker::UserMarker, Id};
use uuid::Uuid;

use crate::bot::App;

use super::types::Member;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, ToSql, FromSql)]
#[postgres(name = "sprint_status")]
pub enum SprintStatus {
	Initial,
	Announced,
	Started,
	Ended,
	Summaried,
}

#[derive(Debug, Clone)]
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

impl Participant {
	fn from_row(row: Row) -> Result<Self> {
		Ok(Self {
			sprint_id: row.try_get("sprint_id").into_diagnostic()?,
			member: row.try_get("member").into_diagnostic()?,
			joined_at: row.try_get("joined_at").into_diagnostic()?,
			words_start: row.try_get("words_start").into_diagnostic()?,
			words_end: row.try_get("words_end").into_diagnostic()?,
		})
	}
}

#[derive(Debug, Clone)]
pub struct Sprint {
	pub id: Uuid,
	pub created_at: DateTime<Utc>,
	pub updated_at: DateTime<Utc>,
	pub cancelled_at: Option<DateTime<Utc>>,
	pub shortid: i32,
	pub starting_at: DateTime<Utc>,
	pub duration: Interval,
	pub status: SprintStatus,
}

impl Sprint {
	fn from_row(row: Row) -> Result<Self> {
		Ok(Self {
			id: row.try_get("id").into_diagnostic()?,
			created_at: row.try_get("created_at").into_diagnostic()?,
			updated_at: row.try_get("updated_at").into_diagnostic()?,
			cancelled_at: row.try_get("cancelled_at").into_diagnostic()?,
			shortid: row.try_get("shortid").into_diagnostic()?,
			starting_at: row.try_get("starting_at").into_diagnostic()?,
			duration: row.try_get("duration").into_diagnostic()?,
			status: row.try_get("status").into_diagnostic()?,
		})
	}

	#[tracing::instrument(skip(app))]
	pub async fn create<TZ>(
		app: App,
		starting_at: DateTime<TZ>,
		duration: chrono::Duration,
		member: Member,
	) -> Result<Self>
	where
		TZ: TimeZone,
	{
		let sprint = app
			.db
			.query_one(
				"INSERT INTO sprints (starting_at, duration) VALUES ($1, $2) RETURNING *",
				&[
					&starting_at.with_timezone(&Utc),
					&Interval::from_duration(duration)
						.ok_or(miette!("could not convert duration to interval"))?,
				],
			)
			.await
			.into_diagnostic()
			.and_then(Self::from_row)
			.wrap_err("db: create sprint")?;

		sprint.join(app, member).await?;
		Ok(sprint)
	}

	#[tracing::instrument(skip(app))]
	pub async fn get(app: App, uuid: Uuid) -> Result<Self> {
		app.db
			.query_one("SELECT * FROM sprints WHERE id = $1", &[&uuid])
			.await
			.into_diagnostic()
			.and_then(Self::from_row)
			.wrap_err("db: get sprint")
	}

	#[tracing::instrument(skip(app))]
	pub async fn get_current(app: App, uuid: Uuid) -> Result<Self> {
		app.db
			.query_one("SELECT * FROM sprints_current WHERE id = $1", &[&uuid])
			.await
			.into_diagnostic()
			.and_then(Self::from_row)
			.wrap_err("db: get current sprint")
	}

	#[tracing::instrument(skip(app))]
	pub async fn participants(&self, app: App) -> Result<Vec<Participant>> {
		app.db
			.query(
				"SELECT * FROM sprint_participants WHERE sprint_id = $1",
				&[&self.id],
			)
			.await
			.into_diagnostic()
			.and_then(|rows| rows.into_iter().map(Participant::from_row).collect())
			.wrap_err("db: get sprint participants")
	}

	#[tracing::instrument(skip(app))]
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

	#[tracing::instrument(skip(app))]
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

	#[tracing::instrument(skip(app))]
	pub async fn join(&self, app: App, member: Member) -> Result<()> {
		app.db
			.query(
				"INSERT INTO sprint_participants (sprint_id, member) VALUES ($1, $2) ON CONFLICT DO NOTHING",
				&[&self.id, &member],
			)
			.await
			.into_diagnostic()
			.wrap_err("db: join sprint")
			.map(drop)
	}

	#[tracing::instrument(skip(app))]
	pub async fn leave(&self, app: App, member: Member) -> Result<()> {
		app.db
			.query(
				"DELETE FROM sprint_participants WHERE sprint_id = $1 AND member = $2",
				&[&self.id, &member],
			)
			.await
			.into_diagnostic()
			.wrap_err("db: leave sprint")
			.map(drop)
	}

	#[tracing::instrument(skip(app))]
	pub async fn set_words(
		&self,
		app: App,
		member: Member,
		words: i32,
		column: &str,
	) -> Result<()> {
		app.db
			.query(
				&format!("UPDATE sprint_participants SET {column} = $3 WHERE sprint_id = $1 AND member = $2"),
				&[&self.id, &member, &words],
			)
			.await
			.into_diagnostic()
			.wrap_err("db: set words for sprint")
			.map(drop)
	}

	pub fn is_cancelled(&self) -> bool {
		self.cancelled_at.is_some()
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

	pub fn ending_at(&self) -> Result<DateTime<Utc>> {
		chrono::Duration::from_std(self.duration())
			.into_diagnostic()
			.map(|dur| self.starting_at + dur)
	}

	pub fn ending_in(&self) -> Option<Duration> {
		let now = Utc::now();
		match self.ending_at() {
			Ok(end) if end > now => Some(Duration::from_secs((end - now).num_seconds() as _)),
			_ => None,
		}
	}
}
