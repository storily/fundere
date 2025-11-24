use miette::{IntoDiagnostic, Result};
use tracing::debug;

use crate::bot::App;

use super::member::Member;

#[derive(Debug, Clone)]
pub struct UserPreference {
	pub member: Member,
	pub timezone: String,
}

impl UserPreference {
	/// Get user preferences for a member, creating with defaults if not exists
	pub async fn get_or_create(app: App, member: Member) -> Result<Self> {
		let row = app
			.db
			.query_opt(
				"INSERT INTO user_preferences (member) VALUES ($1)
				 ON CONFLICT (member) DO UPDATE SET member = EXCLUDED.member
				 RETURNING member, timezone",
				&[&member],
			)
			.await
			.into_diagnostic()?;

		if let Some(row) = row {
			Ok(Self {
				member: row.get(0),
				timezone: row.get(1),
			})
		} else {
			// Fallback to default if somehow no row was returned
			Ok(Self {
				member,
				timezone: "Pacific/Auckland".to_string(),
			})
		}
	}

	/// Get user preferences for a member if they exist
	pub async fn get(app: App, member: Member) -> Result<Option<Self>> {
		let row = app
			.db
			.query_opt(
				"SELECT member, timezone FROM user_preferences WHERE member = $1",
				&[&member],
			)
			.await
			.into_diagnostic()?;

		Ok(row.map(|row| Self {
			member: row.get(0),
			timezone: row.get(1),
		}))
	}

	/// Update the timezone for this user preference
	pub async fn set_timezone(mut self, app: App, timezone: String) -> Result<Self> {
		debug!(?self.member, %timezone, "updating user timezone");

		app.db
			.execute(
				"UPDATE user_preferences SET timezone = $1 WHERE member = $2",
				&[&timezone, &self.member],
			)
			.await
			.into_diagnostic()?;

		self.timezone = timezone;
		Ok(self)
	}

	/// Get the timezone as a chrono_tz::Tz
	pub fn timezone_tz(&self) -> Result<chrono_tz::Tz> {
		self.timezone
			.parse()
			.map_err(|_| miette::miette!("Invalid timezone: {}", self.timezone))
	}
}
