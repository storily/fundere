use std::str::FromStr;

use chrono::{naive::NaiveTime, Duration};
use miette::{IntoDiagnostic, Result};

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
			NaiveTime::from_hms(1, 23, 45)
		);
		assert_eq!(
			parse_when("1:23:45").unwrap(),
			NaiveTime::from_hms(1, 23, 45)
		);
	}

	#[test]
	fn parses_times_without_seconds() {
		assert_eq!(parse_when("01:23").unwrap(), NaiveTime::from_hms(1, 23, 0));
		assert_eq!(parse_when("1:23").unwrap(), NaiveTime::from_hms(1, 23, 0));
	}
}
