use std::{ops::Sub, str::FromStr};

use chrono::{naive::NaiveTime, DateTime, Duration, Timelike, Utc};
use miette::{miette, IntoDiagnostic, Result};

pub trait ChronoDurationExt {
	fn positive_or(self, default: Duration) -> Duration;
	fn saturating_sub(self, other: Self) -> Duration;

	fn saturating_sub_std(self, other: Self) -> std::time::Duration
	where
		Self: Sized,
		Self: Sub,
	{
		// UNWRAP: it will always be zero or above
		self.saturating_sub(other).to_std().unwrap()
	}

	fn round_to_seconds(self) -> std::time::Duration;
}

impl ChronoDurationExt for Duration {
	fn positive_or(self, default: Duration) -> Duration {
		if self > Self::zero() {
			self
		} else {
			default
		}
	}

	fn saturating_sub(self, other: Self) -> Duration {
		(self - other).positive_or(Duration::zero())
	}

	fn round_to_seconds(self) -> std::time::Duration {
		std::time::Duration::from_secs(self.num_milliseconds().max(0) as u64 / 1000)
	}
}

pub trait ChronoDateTimeExt {
	/// Duration since the given time, or None if it's in the future.
	fn elapsed(&self) -> Result<Option<std::time::Duration>>;

	/// True if the date is a day away.
	fn over_a_day_away(&self) -> bool;

	/// Format as discord dynamic timestamp.
	///
	/// Format chars (note they're also locale-dependent, shown here US-format):
	/// - `R` for relative
	/// - `t` for short time (12:01 pm)
	/// - `T` for long time (12:01:39 pm)
	/// - `d` for short date (1/04/23)
	/// - `D` for long date (1 April 2023)
	/// - `f` for long datetime (1 April 2023 at 12:01 pm)
	/// - `F` for long datetime with day of week (Saturday, 1 April 2023 at 12:01 pm)
	fn discord_format(&self, format: char) -> String;
}

impl ChronoDateTimeExt for DateTime<Utc> {
	fn elapsed(&self) -> Result<Option<std::time::Duration>> {
		let since = self.signed_duration_since(Utc::now());
		if since > Duration::zero() {
			since.to_std().map(Some).into_diagnostic()
		} else {
			Ok(None)
		}
	}

	fn over_a_day_away(&self) -> bool {
		self.signed_duration_since(Utc::now()).num_days().abs() >= 1
	}

	fn discord_format(&self, format: char) -> String {
		self.format(&format!("<t:%s:{format}>")).to_string()
	}
}

pub fn parse_when_relative_to(now: NaiveTime, s: &str) -> Result<NaiveTime> {
	if s.to_ascii_lowercase() == "now" {
		return Ok(now);
	}

	if let Ok(minutes) = u8::from_str(s) {
		return Ok(now + Duration::minutes(minutes as _));
	}

	if let Some(seconds) = s
		.strip_suffix(['s', 'S'])
		.and_then(|s| u16::from_str(s).ok())
	{
		return Ok(now + Duration::seconds(seconds as _));
	}

	if let Some(minutes) = s
		.strip_suffix(['m', 'M'])
		.and_then(|s| u16::from_str(s).ok())
	{
		return Ok(now + Duration::minutes(minutes as _));
	}

	if let Some(hours) = s
		.strip_suffix(['h', 'H'])
		.and_then(|s| u16::from_str(s).ok())
	{
		return Ok(now + Duration::hours(hours as _));
	}

	if let Some(minutes) = s.strip_prefix(':').and_then(|s| u32::from_str(s).ok()) {
		return NaiveTime::from_hms_opt(
			if minutes <= now.minute() {
				(now.hour() + 1) % 24
			} else {
				now.hour()
			},
			minutes,
			0,
		)
		.ok_or_else(|| miette!("invalid minutes"));
	}

	if let Ok(time) = NaiveTime::parse_from_str(s, "%H:%M:%S") {
		return Ok(time);
	}

	if let Ok(time) = NaiveTime::parse_from_str(s, "%_H:%M:%S") {
		return Ok(time);
	}

	if let Ok(time) = NaiveTime::parse_from_str(s, "%_H:%M") {
		return Ok(time);
	}

	NaiveTime::parse_from_str(s, "%H:%M").into_diagnostic()
}

#[cfg(test)]
mod test {
	use chrono::{naive::NaiveTime, DateTime, Duration, Utc};
	use chrono_tz::{Pacific, Tz};
	use miette::Result;

	use super::parse_when_relative_to;

	fn now_in_tz() -> DateTime<Tz> {
		let now = Utc::now();
		now.with_timezone(&Pacific::Auckland)
	}

	fn parse_when(s: &str) -> Result<NaiveTime> {
		let now = now_in_tz().time();
		parse_when_relative_to(now, s)
	}

	#[test]
	fn parses_now() {
		let now = now_in_tz().time();
		assert_eq!(parse_when_relative_to(now, "now").unwrap(), now);
		assert_eq!(parse_when_relative_to(now, "NOW").unwrap(), now);
		assert_eq!(parse_when_relative_to(now, "Now").unwrap(), now);
	}

	#[test]
	fn parses_bare_numbers_as_minutes() {
		let now = now_in_tz().time();
		assert_eq!(
			parse_when_relative_to(now, "42").unwrap(),
			now + Duration::minutes(42)
		);
	}

	#[test]
	fn parses_s_suffixed_numbers_as_seconds() {
		let now = now_in_tz().time();
		assert_eq!(
			parse_when_relative_to(now, "1s").unwrap(),
			now + Duration::seconds(1)
		);
		assert_eq!(
			parse_when_relative_to(now, "23S").unwrap(),
			now + Duration::seconds(23)
		);
	}

	#[test]
	fn parses_m_suffixed_numbers_as_minutes() {
		let now = now_in_tz().time();
		assert_eq!(
			parse_when_relative_to(now, "1m").unwrap(),
			now + Duration::minutes(1)
		);
		assert_eq!(
			parse_when_relative_to(now, "23M").unwrap(),
			now + Duration::minutes(23)
		);
	}

	#[test]
	fn parses_h_suffixed_numbers_as_hours() {
		let now = now_in_tz().time();
		assert_eq!(
			parse_when_relative_to(now, "1h").unwrap(),
			now + Duration::hours(1)
		);
		assert_eq!(
			parse_when_relative_to(now, "23H").unwrap(),
			now + Duration::hours(23)
		);
	}

	#[test]
	fn parses_times_with_seconds() {
		assert_eq!(
			parse_when("01:23:45").unwrap(),
			NaiveTime::from_hms_opt(1, 23, 45).unwrap()
		);
		assert_eq!(
			parse_when("1:23:45").unwrap(),
			NaiveTime::from_hms_opt(1, 23, 45).unwrap()
		);
	}

	#[test]
	fn parses_times_without_seconds() {
		assert_eq!(
			parse_when("01:23").unwrap(),
			NaiveTime::from_hms_opt(1, 23, 0).unwrap()
		);
		assert_eq!(
			parse_when("1:23").unwrap(),
			NaiveTime::from_hms_opt(1, 23, 0).unwrap()
		);
	}

	#[test]
	fn parses_times_without_hours() {
		assert_eq!(
			parse_when_relative_to(NaiveTime::from_hms_opt(1, 23, 0).unwrap(), ":30").unwrap(),
			NaiveTime::from_hms_opt(1, 30, 0).unwrap()
		);
		assert_eq!(
			parse_when_relative_to(NaiveTime::from_hms_opt(1, 23, 0).unwrap(), ":20").unwrap(),
			NaiveTime::from_hms_opt(2, 20, 0).unwrap()
		);
		assert_eq!(
			parse_when_relative_to(NaiveTime::from_hms_opt(23, 45, 0).unwrap(), ":12").unwrap(),
			NaiveTime::from_hms_opt(0, 12, 0).unwrap()
		);
	}
}
