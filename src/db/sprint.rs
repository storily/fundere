use std::fmt::Debug;

use chrono::{DateTime, Duration, TimeZone, Utc};
use futures_util::future::try_join_all;
use humantime::{format_duration, FormattedDuration};
use itertools::Itertools;
use miette::{miette, Context, IntoDiagnostic, Result};
use pg_interval::Interval;
use postgres_types::{FromSql, ToSql};
use tokio_postgres::Row;
use tracing::debug;
use twilight_mention::{fmt::MentionFormat, Mention};
use twilight_model::id::{marker::UserMarker, Id};
use uuid::Uuid;

use crate::bot::{
	utils::time::{ChronoDateTimeExt, ChronoDurationExt},
	App,
};

use super::{member::Member, message::Message};

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

	pub fn words_written(&self) -> Option<i32> {
		if let (Some(start), Some(end)) = (self.words_start, self.words_end) {
			Some(end - start)
		} else {
			None
		}
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
	pub interaction_token: String,
	pub announce: Option<Message>,
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
			interaction_token: row.try_get("interaction_token").into_diagnostic()?,
			announce: row.try_get("announce").into_diagnostic()?,
		})
	}

	#[tracing::instrument(skip(app))]
	pub async fn create<TZ>(
		app: App,
		starting_at: DateTime<TZ>,
		duration: Duration,
		interaction_token: &str,
		member: Member,
	) -> Result<Self>
	where
		TZ: TimeZone,
	{
		let sprint = app
			.db
			.query_one(
				"INSERT INTO sprints (starting_at, duration, interaction_token) VALUES ($1, $2, $3) RETURNING *",
				&[
					&starting_at.with_timezone(&Utc),
					&Interval::from_duration(duration)
						.ok_or(miette!("could not convert duration to interval"))?,
					&interaction_token,
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
	pub async fn get_from_shortid(app: App, shortid: i32) -> Result<Self> {
		app.db
			.query_one("SELECT * FROM sprints WHERE shortid = $1", &[&shortid])
			.await
			.into_diagnostic()
			.and_then(Self::from_row)
			.wrap_err("db: get sprint from shortid")
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
	pub async fn get_all_current(app: App) -> Result<Vec<Self>> {
		app.db
			.query("SELECT * FROM sprints_current", &[])
			.await
			.into_diagnostic()
			.and_then(|rows| rows.into_iter().map(Self::from_row).collect())
			.wrap_err("db: get current sprints")
	}

	#[tracing::instrument(skip(app))]
	pub async fn get_all_finished_but_not_ended(app: App) -> Result<Vec<Self>> {
		app.db
			.query("SELECT * FROM sprints_finished_but_not_ended", &[])
			.await
			.into_diagnostic()
			.and_then(|rows| rows.into_iter().map(Self::from_row).collect())
			.wrap_err("db: get sprints that are finished but not ended (nor summaried)")
	}

	#[tracing::instrument(skip(app))]
	pub async fn get_all_finished_but_not_summaried(app: App) -> Result<Vec<Self>> {
		app.db
			.query("SELECT * FROM sprints_finished_but_not_summaried", &[])
			.await
			.into_diagnostic()
			.and_then(|rows| rows.into_iter().map(Self::from_row).collect())
			.wrap_err("db: get sprints that are finished but not summaried")
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
			.wrap_err("db: get all sprint participants")
	}

	#[tracing::instrument(skip(app))]
	pub async fn participant(&self, app: App, member: Member) -> Result<Participant> {
		app.db
			.query_one(
				"SELECT * FROM sprint_participants WHERE sprint_id = $1 AND (member) = $2::member",
				&[&self.id, &member],
			)
			.await
			.into_diagnostic()
			.and_then(Participant::from_row)
			.wrap_err("db: get one sprint participant")
	}

	#[tracing::instrument(skip(app))]
	pub async fn all_participants_have_ending_words(&self, app: App) -> Result<bool> {
		let unfinished: i64 = app.db
			.query_one(
				"SELECT count(*) AS unfinished FROM sprint_participants WHERE sprint_id = $1 AND words_end IS NULL",
				&[&self.id],
			)
			.await
			.into_diagnostic()
			.and_then(|count| count.try_get("unfinished").into_diagnostic())
			.wrap_err("db: get count of participants without ending words")?;

		Ok(unfinished <= 0)
	}

	#[tracing::instrument(skip(app))]
	pub async fn update_status(&self, app: App, status: SprintStatus) -> Result<()> {
		app.db
			.query(
				"UPDATE sprints SET status = $2, updated_at = CURRENT_TIMESTAMP WHERE id = $1",
				&[&self.id, &status],
			)
			.await
			.into_diagnostic()
			.wrap_err("db: update sprint status")
			.map(drop)
	}

	#[tracing::instrument(skip(app))]
	pub async fn set_announce(&self, app: App, message: Message) -> Result<()> {
		app.db
			.query(
				"UPDATE sprints SET announce = $2, updated_at = CURRENT_TIMESTAMP WHERE id = $1",
				&[&self.id, &message],
			)
			.await
			.into_diagnostic()
			.wrap_err("db: set sprint announce")
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
				"DELETE FROM sprint_participants WHERE sprint_id = $1 AND (member) = $2::member",
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
				&format!("UPDATE sprint_participants SET {column} = $3 WHERE sprint_id = $1 AND (member) = $2::member"),
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
		Duration::seconds(
			(self.duration.days as i64 + self.duration.months as i64 * 31)
				* 24 * 60 * 60 * 1_000_000
				+ self.duration.microseconds / (1_000_000),
		)
	}

	/// Formatted duration, excluding sign
	pub fn formatted_duration(&self) -> FormattedDuration {
		let duration = self.duration();
		format_duration(duration.round_to_seconds())
	}

	pub fn starting_in(&self) -> Duration {
		let now = Utc::now();
		self.starting_at - now
	}

	pub fn warning_in(&self) -> Duration {
		self.starting_in().saturating_sub(Duration::seconds(30))
	}

	pub fn ending_at(&self) -> DateTime<Utc> {
		self.starting_at + self.duration()
	}

	pub fn ending_in(&self) -> Duration {
		let now = Utc::now();
		self.ending_at() - now
	}

	pub async fn status_text(&self, app: App, announce: bool) -> Result<String> {
		let starting_at_ts =
			self.starting_at
				.discord_format(if self.starting_at.over_a_day_away() {
					'f'
				} else {
					'T'
				});

		let shortid = self.shortid;
		let duration = self.formatted_duration();

		let starting_in_ts = if self.starting_in() <= Duration::zero() {
			"now".into()
		} else {
			self.starting_at.discord_format('R')
		};

		Ok(if announce {
			format!(
				"⏱️  New sprint! `{shortid}` is starting {starting_in_ts} (at {starting_at_ts}), going for {duration}."
			)
		} else {
			let participants = try_join_all(
				self.participants(app.clone())
					.await?
					.into_iter()
					.map(|p| p.member.name(app.clone())),
			)
			.await?
			.join(", ");
			format!(
				"⏱️ Sprint `{shortid}` starts at {starting_at_ts}, lasts for {duration}, with {participants}."
			)
		})
	}

	#[tracing::instrument(skip(app))]
	pub async fn summary_text(&self, app: App) -> Result<String> {
		let started_at = self
			.starting_at
			.discord_format(if self.starting_at.over_a_day_away() {
				'f'
			} else {
				'T'
			});

		let shortid = self.shortid;
		let duration = self.formatted_duration();
		let minutes = self.duration().num_minutes();

		let participants = self.participants(app.clone()).await?;
		let mut summaries = Vec::with_capacity(participants.len());
		for p in participants {
			let name = p.member.name(app.clone()).await?;
			let words = p
				.words_end
				.map_or(0, |end| end - p.words_start.unwrap_or(0));
			let wpm = (words as f64) / (minutes as f64);
			summaries.push((name, words, wpm));
		}

		summaries.sort_by_key(|(_, w, _)| *w);
		debug!(?summaries, "sprint summary");
		let summary = summaries
			.into_iter()
			.map(|(name, words, wpm)| {
				format!(
					"_{name}_: **{words}** words (**{wpm:.1}** words per minute)",
					name = name.replace('_', "\\_")
				)
			})
			.join("\n");

		Ok(format!(
			"🧮 Sprint `{shortid}`, {duration}, started at {started_at}:\n{summary}"
		))
	}
}
