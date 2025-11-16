use std::fmt::Debug;

use chrono::{DateTime, Utc};
use miette::{Context, IntoDiagnostic, Result};
use secret_vault_value::SecretValue;
use tokio_postgres::Row;
use uuid::Uuid;

use crate::bot::App;
use crate::trackbear::TrackbearClient;

use super::member::Member;

#[expect(dead_code, reason = "unused fields")]
#[derive(Debug, Clone)]
pub struct TrackbearLogin {
	pub id: Uuid,
	pub created_at: DateTime<Utc>,
	pub updated_at: DateTime<Utc>,
	pub member: Member,
	pub api_key: SecretValue,
	pub ask_me: bool,
}

impl TrackbearLogin {
	fn from_row(row: Row) -> Result<Self> {
		Ok(Self {
			id: row.try_get("id").into_diagnostic()?,
			created_at: row.try_get("created_at").into_diagnostic()?,
			updated_at: row.try_get("updated_at").into_diagnostic()?,
			member: row.try_get("member").into_diagnostic()?,
			api_key: row.try_get::<_, &str>("api_key").into_diagnostic()?.into(),
			ask_me: row.try_get("ask_me").into_diagnostic()?,
		})
	}

	#[tracing::instrument(skip(app))]
	pub async fn create(app: App, member: Member, api_key: SecretValue) -> Result<Self> {
		app.db
			.query_one(
				"
				INSERT INTO trackbear_logins (member, api_key)
				VALUES ($1, $2)
				ON CONFLICT (member) DO UPDATE SET
					api_key = EXCLUDED.api_key
				RETURNING *
				",
				&[&member, &api_key.as_sensitive_str()],
			)
			.await
			.into_diagnostic()
			.and_then(Self::from_row)
			.wrap_err("db: create trackbear login")
	}

	#[tracing::instrument(skip(app))]
	pub async fn get(app: App, uuid: Uuid) -> Result<Option<Self>> {
		app.db
			.query("SELECT * FROM trackbear_logins WHERE id = $1", &[&uuid])
			.await
			.into_diagnostic()
			.and_then(|mut rows| {
				if let Some(row) = rows.pop() {
					Self::from_row(row).map(Some)
				} else {
					Ok(None)
				}
			})
			.wrap_err("db: get trackbear login")
	}

	#[tracing::instrument(skip(app))]
	pub async fn get_for_member(app: App, member: Member) -> Result<Option<Self>> {
		app.db
			.query(
				"SELECT * FROM trackbear_logins WHERE (member) = $1::member",
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
			.wrap_err("db: get trackbear login for member")
	}

	#[tracing::instrument(skip(app))]
	pub async fn update(&mut self, app: App, api_key: SecretValue) -> Result<()> {
		self.api_key = api_key.clone();
		app.db
			.query(
				"
				UPDATE trackbear_logins SET
					api_key = $2,
					updated_at = CURRENT_TIMESTAMP
				WHERE id = $1
				",
				&[&self.id, &api_key.as_sensitive_str()],
			)
			.await
			.into_diagnostic()
			.wrap_err("db: update trackbear login")
			.map(drop)
	}

	#[tracing::instrument(skip(app))]
	pub async fn delete(self, app: App) -> Result<()> {
		app.db
			.query("DELETE FROM trackbear_logins WHERE id = $1", &[&self.id])
			.await
			.into_diagnostic()
			.wrap_err("db: delete trackbear login")
			.map(drop)
	}

	#[tracing::instrument(skip(app))]
	pub async fn ask_me(&mut self, app: App, ask_me: bool) -> Result<()> {
		self.ask_me = ask_me;
		app.db
			.query(
				"
				UPDATE trackbear_logins SET
					ask_me = $2,
					updated_at = CURRENT_TIMESTAMP
				WHERE id = $1
				",
				&[&self.id, &ask_me],
			)
			.await
			.into_diagnostic()
			.wrap_err("db: update trackbear ask me")
			.map(drop)
	}

	#[tracing::instrument]
	pub async fn client(&self) -> Result<TrackbearClient> {
		let client = TrackbearClient::new(self.api_key.clone())?;
		client.validate().await?;
		Ok(client)
	}
}
