use std::fmt::Debug;

use chrono::{DateTime, Utc};
use miette::{Context, IntoDiagnostic, Result};
use nanowrimo::NanoClient;
use secret_vault_value::SecretValue;
use tokio_postgres::Row;
use uuid::Uuid;

use crate::bot::App;

use super::member::Member;

#[derive(Debug, Clone)]
pub struct NanowrimoLogin {
	pub id: Uuid,
	pub created_at: DateTime<Utc>,
	pub updated_at: DateTime<Utc>,
	pub member: Member,
	pub username: String,
	pub password: SecretValue,
}

impl NanowrimoLogin {
	fn from_row(row: Row) -> Result<Self> {
		Ok(Self {
			id: row.try_get("id").into_diagnostic()?,
			created_at: row.try_get("created_at").into_diagnostic()?,
			updated_at: row.try_get("updated_at").into_diagnostic()?,
			member: row.try_get("member").into_diagnostic()?,
			username: row.try_get("username").into_diagnostic()?,
			password: row.try_get::<_, &str>("password").into_diagnostic()?.into(),
		})
	}

	#[tracing::instrument(skip(app))]
	pub async fn create(
		app: App,
		member: Member,
		username: &str,
		password: SecretValue,
	) -> Result<Self> {
		app.db
			.query_one(
				"
				INSERT INTO nanowrimo_logins (member, username, password)
				VALUES ($1, $2, $3)
				ON CONFLICT (member) DO UPDATE SET
					username = EXCLUDED.username,
					password = EXCLUDED.password
				RETURNING *
				",
				&[&member, &username, &password.as_sensitive_str()],
			)
			.await
			.into_diagnostic()
			.and_then(Self::from_row)
			.wrap_err("db: create nanowrimo login")
	}

	#[tracing::instrument(skip(app))]
	pub async fn get(app: App, uuid: Uuid) -> Result<Self> {
		app.db
			.query_one("SELECT * FROM nanowrimo_logins WHERE id = $1", &[&uuid])
			.await
			.into_diagnostic()
			.and_then(Self::from_row)
			.wrap_err("db: get nanowrimo login")
	}

	#[tracing::instrument(skip(app))]
	pub async fn get_default(app: App) -> Option<Self> {
		Self::get(app, Uuid::nil()).await.ok()
	}

	#[tracing::instrument(skip(app))]
	pub async fn get_for_member(app: App, member: Member) -> Result<Self> {
		app.db
			.query_one(
				"SELECT * FROM nanowrimo_logins WHERE member = $1",
				&[&member],
			)
			.await
			.into_diagnostic()
			.and_then(Self::from_row)
			.wrap_err("db: get nanowrimo login for member")
	}

	#[tracing::instrument(skip(app))]
	pub async fn update(&mut self, app: App, username: &str, password: SecretValue) -> Result<()> {
		self.username = username.into();
		self.password = password.clone();
		app.db
			.query(
				"
				UPDATE nanowrimo_logins SET
					username = $2,
					password = $3,
					updated_at = CURRENT_TIMESTAMP
				WHERE id = $1
				",
				&[&self.id, &username, &password.as_sensitive_str()],
			)
			.await
			.into_diagnostic()
			.wrap_err("db: update nanowrimo login")
			.map(drop)
	}

	#[tracing::instrument(skip(app))]
	pub async fn delete(self, app: App) -> Result<()> {
		app.db
			.query("DELETE FROM nanowrimo_logins WHERE id = $1", &[&self.id])
			.await
			.into_diagnostic()
			.wrap_err("db: delete nanowrimo login")
			.map(drop)
	}

	#[tracing::instrument]
	pub async fn client(&self) -> Result<NanoClient> {
		NanoClient::new_user(&self.username, &self.password.as_sensitive_str())
			.await
			.into_diagnostic()
	}
}
