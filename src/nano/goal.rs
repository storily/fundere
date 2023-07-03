use std::fmt::Debug;

use chrono::{Duration, Timelike, Utc};
use chrono_tz::Tz;
use nanowrimo::{EventType, ProjectChallengeData as Data};
use tracing::debug;

#[derive(Clone, Debug)]
pub struct Goal {
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
	pub fn new(timezone: Tz, data: Data) -> Self {
		Self { data, timezone }
	}

	pub fn set(&mut self, new: u64) {
		self.data.goal = new;
	}

	pub fn is_november(&self) -> bool {
		self.data.event_type == EventType::NanoWrimo
	}

	pub fn is_current(&self) -> bool {
		let today = Utc::now().with_timezone(&self.timezone).date_naive();
		today >= self.data.starts_at && today <= self.data.ends_at
	}

	pub fn length(&self) -> Duration {
		self.data.ends_at.signed_duration_since(self.data.starts_at)
	}

	#[allow(dead_code)] // hard to get right, leaving just in case
	pub fn time_left(&self) -> Option<Duration> {
		if self.is_current() {
			let today = Utc::now().with_timezone(&self.timezone).date_naive();
			Some(self.data.ends_at.signed_duration_since(today))
		} else {
			None
		}
	}

	pub fn time_gone(&self) -> Option<Duration> {
		if self.is_current() {
			let today = Utc::now().with_timezone(&self.timezone).date_naive();
			Some(today.signed_duration_since(self.data.starts_at))
		} else {
			None
		}
	}

	pub fn progress(&self) -> Option<GoalProgress> {
		let count = float(self.data.current_count);
		let goal = float(self.data.goal);
		let now = Utc::now().with_timezone(&self.timezone);

		let today = now.date_naive();
		if self.data.starts_at > today {
			return None;
		}

		let length = self.length();
		let gone = self.time_gone();

		let whole_days = length.num_days();
		let full_days = whole_days + 1;
		let per_day = goal / float(full_days);

		let days_gone = gone.map(|d| d.num_days()).unwrap_or(0);
		let target_day = (days_gone + 1).min(length.num_days());
		let target_today = float(target_day) * per_day;

		let secs_from_midnight = now.num_seconds_from_midnight();
		let secs_total: u32 = 60 * 24 * 24;
		let target_live = (target_today
			- (per_day * float(secs_total.saturating_sub(secs_from_midnight)) / float(secs_total)))
		.min(target_today);

		let diff_today = count - target_today;
		let diff_live = count - target_live;

		debug!(
			?length,
			?whole_days,
			?full_days,
			?per_day,
			?secs_from_midnight,
			?gone,
			?days_gone,
			?target_day,
			?today,
			?now,
			?goal,
			?count,
			?target_today,
			?diff_today,
			?target_live,
			?diff_live,
			"progress debug"
		);

		Some(GoalProgress {
			percent: 100.0 * count / goal,
			today: GoalTarget {
				target: target_today.round() as _,
				diff: diff_today.round() as _,
			},
			live: GoalTarget {
				target: target_live.round() as _,
				diff: diff_live.round() as _,
			},
		})
	}
}

fn float<T>(n: T) -> f64
where
	u32: TryFrom<T>,
{
	f64::from(u32::try_from(n).unwrap_or(u32::MAX))
}
