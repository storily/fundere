use std::fmt::{Debug, Write};

use chrono::{DateTime, Utc};
use miette::{Context, IntoDiagnostic, Result};
use tokio_postgres::Row;
use tracing::debug;
use uuid::Uuid;

use crate::{bot::App, nano::project::Project as NanoProject};

use super::member::Member;

#[derive(Debug, Clone)]
pub struct Project {
	pub id: Uuid,
	pub created_at: DateTime<Utc>,
	pub updated_at: DateTime<Utc>,
	pub member: Member,
	pub nano_id: u64,
	pub goal: Option<u64>,
}

impl Project {
	fn from_row(row: Row) -> Result<Self> {
		Ok(Self {
			id: row.try_get("id").into_diagnostic()?,
			created_at: row.try_get("created_at").into_diagnostic()?,
			updated_at: row.try_get("updated_at").into_diagnostic()?,
			member: row.try_get("member").into_diagnostic()?,
			nano_id: row.try_get::<_, i64>("nano_id").into_diagnostic()? as _,
			goal: row
				.try_get::<_, Option<i32>>("goal")
				.into_diagnostic()?
				.map(|n| n as _),
		})
	}

	#[tracing::instrument(skip(app))]
	async fn create(app: App, member: Member, id: u64) -> Result<Self> {
		app.db
			.query_one(
				"
				INSERT INTO projects (member, nano_id)
				VALUES ($1, $2)
				ON CONFLICT (member) DO UPDATE SET nano_id = EXCLUDED.nano_id
				RETURNING *
				",
				&[&member, &(id as i64)],
			)
			.await
			.into_diagnostic()
			.and_then(Self::from_row)
			.wrap_err("db: create project")
	}

	#[tracing::instrument(skip(app))]
	pub async fn create_or_replace(app: App, member: Member, id: u64) -> Result<Self> {
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

	#[tracing::instrument(skip(app))]
	pub async fn set_goal(&self, app: App, goal: u32) -> Result<()> {
		app.db
			.query(
				"UPDATE projects SET goal = $2, updated_at = CURRENT_TIMESTAMP WHERE id = $1",
				&[&self.id, &i32::try_from(goal).into_diagnostic()?],
			)
			.await
			.into_diagnostic()
			.wrap_err("db: set project goal")
			.map(drop)
	}

	#[tracing::instrument(skip(app))]
	pub async fn unset_goal(&self, app: App) -> Result<()> {
		app.db
			.query(
				"UPDATE projects SET goal = NULL, updated_at = CURRENT_TIMESTAMP WHERE id = $1",
				&[&self.id],
			)
			.await
			.into_diagnostic()
			.wrap_err("db: unset project goal")
			.map(drop)
	}

	pub async fn fetch(&self, app: App) -> Result<NanoProject> {
		NanoProject::fetch(app.clone(), self.member, self.nano_id).await
	}

	pub async fn show_text(&self, app: App) -> Result<String> {
		let proj = self.fetch(app.clone()).await?;
		let title = proj.title();
		let count = proj.wordcount();

		let mut deets = String::new();

		if let Some(mut goal) = proj.current_goal().cloned() {
			if let Some(over) = self.goal {
				goal.set(over);
			}

			if let Some(prog) = goal.progress() {
				write!(deets, "{:.2}% done", prog.percent).ok();
				if prog.percent < 100.0 {
					if prog.today.diff == 0 {
						write!(deets, ", on track").ok();
						if prog.live.diff != 0 {
							write!(deets, " / {live} live", live = tracking(prog.live.diff)).ok();
						}
					} else {
						write!(
							deets,
							", {today} today / {live} live",
							today = tracking(prog.today.diff),
							live = tracking(prog.live.diff)
						)
						.ok();
					}
				}

				if !(goal.is_november() && goal.data.goal == 50_000) {
					write!(deets, ", {} goal", numberk(goal.data.goal as _)).ok();
				}
			}
		} else {
			write!(deets, "no goal").ok();
		}

		Ok(format!("“{title}”: **{count}** words ({deets})"))
	}
}

fn numberk(n: i64) -> String {
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

fn tracking(mut diff: i64) -> String {
	let state = if diff < 0 {
		diff = diff.abs();
		"behind"
	} else {
		"ahead"
	};

	format!("{} {state}", numberk(diff))
}
