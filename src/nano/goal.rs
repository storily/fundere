use std::fmt::Debug;

use chrono::{DateTime, Utc};
use chrono_tz::Tz;
use miette::{Context, IntoDiagnostic, Result};
use nanowrimo::{
	NanoKind, Object, ProjectChallengeData as Data, ProjectChallengeObject, ProjectData,
	ProjectObject,
};
use tokio_postgres::Row;
use tracing::debug;
use uuid::Uuid;

use crate::{
	bot::App,
	db::{member::Member, nanowrimo_login::NanowrimoLogin},
};

#[derive(Debug)]
pub struct Goal {
	app: App,
	pub data: Data,
	timezone: Tz,
}

impl Goal {
	pub fn new(app: App, timezone: Tz, data: Data) -> Self {
		Self {
			app,
			data,
			timezone,
		}
	}

	pub fn is_current(&self) -> bool {
		let today = Utc::now().with_timezone(&self.timezone).date_naive();
		today >= self.data.starts_at && today <= self.data.ends_at
	}
}
