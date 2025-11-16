use std::fmt::{Debug, Write};

use chrono::{DateTime, Utc};
use miette::{Context, IntoDiagnostic, Result};
use tokio_postgres::Row;
use tracing::debug;
use uuid::Uuid;

use crate::{
	bot::{
		utils::pretties::{palindrome_after, Effect},
		App,
	},
	db::trackbear_login::TrackbearLogin,
	trackbear::Project as TrackbearProject,
};

use super::member::Member;

#[expect(dead_code, reason = "unused fields")]
#[derive(Debug, Clone)]
pub struct Project {
	pub id: Uuid,
	pub created_at: DateTime<Utc>,
	pub updated_at: DateTime<Utc>,
	pub member: Member,
	pub trackbear_id: i64,
}

impl Project {
	fn from_row(row: Row) -> Result<Self> {
		Ok(Self {
			id: row.try_get("id").into_diagnostic()?,
			created_at: row.try_get("created_at").into_diagnostic()?,
			updated_at: row.try_get("updated_at").into_diagnostic()?,
			member: row.try_get("member").into_diagnostic()?,
			trackbear_id: row.try_get::<_, i64>("trackbear_id").into_diagnostic()?,
		})
	}

	#[tracing::instrument(skip(app))]
	pub async fn create(app: App, member: Member, id: i64) -> Result<Self> {
		app.db
			.query_one(
				"
				INSERT INTO projects (member, trackbear_id)
				VALUES ($1, $2)
				ON CONFLICT (member) DO UPDATE SET trackbear_id = EXCLUDED.trackbear_id
				RETURNING *
				",
				&[&member, &id],
			)
			.await
			.into_diagnostic()
			.and_then(Self::from_row)
			.wrap_err("db: create project")
	}

	#[tracing::instrument(skip(app))]
	pub async fn create_or_replace(app: App, member: Member, id: i64) -> Result<Self> {
		if let Some(project) = Self::get_for_member(app.clone(), member).await? {
			debug!(?project.id, "deleting project before replace");
			app.db
				.query("DELETE FROM projects WHERE id = $1", &[&project.id])
				.await
				.into_diagnostic()
				.wrap_err("db: delete project")?;
		}

		Self::create(app, member, id).await
	}

	#[tracing::instrument(skip(app))]
	pub async fn get(app: App, uuid: Uuid) -> Result<Option<Self>> {
		app.db
			.query("SELECT * FROM projects WHERE id = $1", &[&uuid])
			.await
			.into_diagnostic()
			.and_then(|mut rows| {
				if let Some(row) = rows.pop() {
					Self::from_row(row).map(Some)
				} else {
					Ok(None)
				}
			})
			.wrap_err("db: get project")
	}

	#[tracing::instrument(skip(app))]
	pub async fn get_for_member(app: App, member: Member) -> Result<Option<Self>> {
		app.db
			.query(
				"SELECT * FROM projects WHERE (member) = $1::member",
				&[&member],
			)
			.await
			.into_diagnostic()
			.and_then(|mut rows| {
				if let Some(row) = rows.pop() {
					Self::from_row(row).map(Some)
				} else {
					Ok(None)
				}
			})
			.wrap_err("db: get project for member")
	}

	pub async fn fetch(&self, app: App) -> Result<TrackbearProject> {
		let login = TrackbearLogin::get_for_member(app.clone(), self.member)
			.await?
			.ok_or_else(|| {
				miette::miette!("No TrackBear login found. Use /trackbear login first.")
			})?;

		let client = login.client().await?;
		TrackbearProject::fetch(&client, self.trackbear_id).await
	}

	pub async fn show_text(&self, app: App) -> Result<String> {
		let proj = self.fetch(app.clone()).await?;
		let title = proj.title();
		let count = proj.word_count();

		let (mut decorated, mut words) = Effect::decorate(count as u64, false);
		let mut deets = String::new();

		if let Some(goal) = proj.current_goal() {
			if let Some(prog) = proj.goal_progress(goal) {
				(decorated, words) = Effect::decorate(count as u64, prog.percent >= 100.0);

				write!(deets, "{:.2}% done", prog.percent).ok();

				if prog.percent < 100.0 {
					write!(deets, ", {}", prog.format_tracking()).ok();
					if prog.words_per_day_to_finish != prog.daily_target {
						write!(
							deets,
							" ({} needed/day)",
							format_count(prog.words_per_day_to_finish)
						)
						.ok();
					}
				}
			}

			if goal.parameters.threshold.count != 50_000 {
				write!(
					deets,
					", {} goal",
					format_count(goal.parameters.threshold.count)
				)
				.ok();
			}
		} else {
			write!(deets, "no goal").ok();
		}

		if !decorated {
			let next_pretty = Effect::on_after(count as u64);
			let next_palindrome = palindrome_after(count as u64);

			if next_pretty == next_palindrome {
				write!(
					deets,
					", {} to next pal",
					next_palindrome.saturating_sub(count as u64)
				)
				.ok();
			} else {
				write!(
					deets,
					", {}/{} to next pretty/pal",
					next_pretty.saturating_sub(count as u64),
					next_palindrome.saturating_sub(count as u64)
				)
				.ok();
			}
		}

		Ok(format!("\"{title}\": **{words}** words ({deets})"))
	}
}

fn format_count(n: i64) -> String {
	if n < 1000 {
		n.to_string()
	} else if n < 10_000 {
		format!("{:.1}k", (n as f64) / 1_000.0)
	} else if n < 1_000_000 {
		format!("{:.0}k", (n as f64) / 1_000.0)
	} else if n < 10_000_000 {
		format!("{:.1}M", (n as f64) / 1_000_000.0)
	} else {
		format!("{:.0}M", (n as f64) / 1_000_000.0)
	}
}
