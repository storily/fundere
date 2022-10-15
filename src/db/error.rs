use std::fmt::Debug;

use chrono::{DateTime, Utc};
use miette::{Context, IntoDiagnostic, Result};
use tokio_postgres::Row;
use uuid::Uuid;

use crate::bot::App;

use super::member::Member;

#[derive(Debug, Clone)]
pub struct Error {
	pub id: Uuid,
	pub created_at: DateTime<Utc>,
	pub member: Member,
	pub message: String,
	pub reported: bool,
}

impl Error {
	fn from_row(row: Row) -> Result<Self> {
		Ok(Self {
			id: row.try_get("id").into_diagnostic()?,
			created_at: row.try_get("created_at").into_diagnostic()?,
			member: row.try_get("member").into_diagnostic()?,
			message: row.try_get("message").into_diagnostic()?,
			reported: row.try_get("reported").into_diagnostic()?,
		})
	}

	#[tracing::instrument(skip(app))]
	pub async fn create(app: App, member: Member, text: &str) -> Result<Self> {
		app.db
			.query_one(
				"INSERT INTO errors (member, message) VALUES ($1, $2) RETURNING *",
				&[&member, &text],
			)
			.await
			.into_diagnostic()
			.and_then(Self::from_row)
			.wrap_err("db: create error")
	}

	#[tracing::instrument(skip(app))]
	pub async fn get(app: App, uuid: Uuid) -> Result<Self> {
		app.db
			.query_one("SELECT * FROM errors WHERE id = $1", &[&uuid])
			.await
			.into_diagnostic()
			.and_then(Self::from_row)
			.wrap_err("db: get error")
	}

	#[tracing::instrument(skip(app))]
	pub async fn set_reported(&self, app: App) -> Result<()> {
		app.db
			.query(
				"UPDATE errors SET reported = true WHERE id = $1",
				&[&self.id],
			)
			.await
			.into_diagnostic()
			.wrap_err("db: get error")
			.map(drop)
	}
}
