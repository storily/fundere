use std::fmt::Debug;

use chrono::{DateTime, Utc};
use miette::{Context, IntoDiagnostic, Result};
use tokio_postgres::Row;
use tracing::debug;
use uuid::Uuid;

use crate::bot::App;

use super::member::Member;

#[derive(Debug, Clone)]
pub struct Project {
	pub id: Uuid,
	pub created_at: DateTime<Utc>,
	pub updated_at: DateTime<Utc>,
	pub member: Member,
	pub nano_id: i64,
	pub goal: Option<i32>,
}

impl Project {
	fn from_row(row: Row) -> Result<Self> {
		Ok(Self {
			id: row.try_get("id").into_diagnostic()?,
			created_at: row.try_get("created_at").into_diagnostic()?,
			updated_at: row.try_get("updated_at").into_diagnostic()?,
			member: row.try_get("member").into_diagnostic()?,
			nano_id: row.try_get("nano_id").into_diagnostic()?,
			goal: row.try_get("goal").into_diagnostic()?,
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
			.query("SELECT * FROM projects WHERE (member) = $1::member", &[&member])
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

	pub async fn fetch_count(&self, _app: App) -> Result<u64> {
		todo!()
	}

	pub async fn fetch_goal(&self, _app: App) -> Result<u64> {
		todo!()
	}

	pub async fn goal(&self, app: App) -> Result<u64> {
		if let Some(goal) = self.goal {
			Ok(u64::try_from(goal).into_diagnostic()?)
		} else {
			self.fetch_goal(app).await
		}
	}

	pub async fn show_text(&self, _app: App) -> Result<String> {
		todo!()
	}
}
