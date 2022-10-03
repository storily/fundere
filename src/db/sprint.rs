use std::{str::FromStr, time::Duration};

use chrono::{DateTime, Utc};
use miette::{ IntoDiagnostic, Result};
use sqlx::{postgres::types::PgInterval, types::Uuid, PgPool};
use strum::{Display, EnumString};

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, EnumString, Display)]
#[strum(serialize_all = "lowercase")]
pub enum SprintStatus {
	Initial,
	Announced,
	Started,
	Ended,
	Summaried,
}

#[derive(Debug, Clone, sqlx::FromRow)]
pub struct Sprint {
	pub id: Uuid,
	pub shortid: i32,
	pub starting_at: DateTime<Utc>,
	pub duration: PgInterval,
	pub status: String,
}

impl Sprint {
	pub async fn update_status(&self, pool: &PgPool, status: SprintStatus) -> Result<()> {
		sqlx::query("UPDATE sprints SET status = $2 WHERE id = $1")
			.bind(self.id)
			.bind(status.to_string())
			.execute(pool)
			.await
			.into_diagnostic()?;
		Ok(())
	}

	pub fn status(&self) -> Result<SprintStatus> {
		SprintStatus::from_str(&self.status).into_diagnostic()
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
