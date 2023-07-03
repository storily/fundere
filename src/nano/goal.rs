use std::fmt::Debug;

use chrono::{DateTime, Datelike, Days, TimeZone, Timelike, Utc};
use chrono_tz::Tz;
use miette::{Context, IntoDiagnostic, Result};
use nanowrimo::{
	EventType, NanoKind, Object, ProjectChallengeData as Data, ProjectChallengeObject, ProjectData,
	ProjectObject, UnitType,
};
use tokio_postgres::Row;
use tracing::debug;
use uuid::Uuid;

use crate::{
	bot::App,
	db::{member::Member, nanowrimo_login::NanowrimoLogin},
};

#[derive(Clone, Debug)]
pub struct Goal {
	app: App,
	pub data: Data,
	timezone: Tz,
}

#[derive(Clone, Debug)]
pub struct GoalProgress {
	pub percent: f64,
	pub today: GoalTarget,
	pub live: GoalTarget,
}

#[derive(Clone, Debug)]
pub struct GoalTarget {
	pub target: u64,
	pub diff: i64,
}

impl Goal {
	pub fn new(app: App, timezone: Tz, data: Data) -> Self {
		Self {
			app,
			data,
			timezone,
		}
	}

	pub fn set(&mut self, new: u64) {
		self.data.goal = new;
	}

	pub fn is_current(&self) -> bool {
		let today = Utc::now().with_timezone(&self.timezone).date_naive();
		today >= self.data.starts_at && today <= self.data.ends_at
	}

	pub fn progress(&self, force_goal: Option<u64>) -> GoalProgress {
		todo!()
	}

	pub fn default_to_this_month(app: App, timezone: Tz) -> Self {
		let today = Utc::now().with_timezone(&timezone);
		let month = today.month();
		let start_of_this_month = zero_time(today.with_day(1))
			.expect("should always be able to find the start of the month");
		let start_of_next_month = zero_time(
			today
				.with_day(1)
				.and_then(|dt| dt.with_month0(if month == 12 { 1 } else { month + 1 })),
		);
		let end_of_this_month = start_of_next_month
			.and_then(|dt| dt.checked_sub_days(Days::new(1)))
			.expect("should always be able to find the end of the month");
		Self::new(
			app,
			timezone,
			Data {
				challenge_id: 0,
				current_count: 0,
				ends_at: end_of_this_month.date_naive(),
				event_type: EventType::NanoWrimo,
				feeling: None,
				goal: 50_000,
				how: None,
				last_recompute: None,
				name: "Default goal".into(),
				project_id: 0,
				speed: None,
				start_count: Some(0),
				starts_at: start_of_this_month.date_naive(),
				streak: None,
				unit_type: UnitType::Words,
				user_id: 0,
				when: None,
				won_at: None,
				writing_location: None,
				writing_type: None,
			},
		)
	}
}

fn zero_time<Tz: TimeZone>(dt: Option<DateTime<Tz>>) -> Option<DateTime<Tz>> {
	dt.and_then(|dt| dt.with_hour(0))
		.and_then(|dt| dt.with_minute(0))
		.and_then(|dt| dt.with_second(0))
		.and_then(|dt| dt.with_nanosecond(0))
}
