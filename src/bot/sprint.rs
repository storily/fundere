use std::str::FromStr;

use chrono::{naive::NaiveTime, Duration, Utc};
use miette::{miette, Context, IntoDiagnostic, Result};
use sqlx::{types::Uuid, Row};
use tracing::{debug, warn};
use twilight_model::application::{
	command::{Command, CommandType},
	interaction::{
		application_command::{CommandData, CommandDataOption, CommandOptionValue},
		Interaction,
	},
};
use twilight_util::builder::command::{
	CommandBuilder, IntegerBuilder, StringBuilder, SubCommandBuilder,
};

use crate::bot::action::SprintAnnounce;

use super::App;

pub fn command(_app: App) -> Result<Command> {
	CommandBuilder::new(
		"sprint",
		"Experimental new-gen wordwar/sprint command",
		CommandType::ChatInput,
	)
	.option({
		let when = StringBuilder::new(
			"when",
			"When to start the sprint, either in clock time (08:30), or in relative time (15m)",
		)
		.required(true);
		let duration = IntegerBuilder::new(
			"duration",
			"Duration of the sprint in minutes (defaults to 15)",
		);
		SubCommandBuilder::new("start", "Create a new sprint")
			.option(when)
			.option(duration)
	})
	.validate()
	.into_diagnostic()
	.map(|cmd| cmd.build())
}

pub async fn handle(app: App, interaction: &Interaction, command_data: &CommandData) -> Result<()> {
	let subcmd = command_data.options.iter().find_map(|opt| {
		if let CommandOptionValue::SubCommand(ref sub) = opt.value {
			Some((opt.name.as_str(), sub.as_slice()))
		} else {
			None
		}
	});

	match subcmd {
		Some(("start", opts)) => sprint_start(app.clone(), interaction, opts)
			.await
			.wrap_err("command: start")?,
		Some((other, _)) => warn!("unhandled sprint subcommand: {other}"),
		_ => todo!("handle bare sprint command?"),
	}

	Ok(())
}

async fn sprint_start(
	app: App,
	interaction: &Interaction,
	options: &[CommandDataOption],
) -> Result<()> {
	let duration = get_integer(options, "duration").unwrap_or(15);
	if duration <= 0 {
		return Err(miette!("duration must be positive"));
	}
	let duration = Duration::minutes(duration);

	// TODO: derive timezone or offset from calling user
	let now = Utc::now().with_timezone(&chrono_tz::Pacific::Auckland);

	let when = parse_when_relative_to(now.time(), get_string(options, "when").unwrap_or("15m"))?;

	let now_with_time = now.date().and_time(when).ok_or(miette!("invalid time"))?;
	let starting = if now_with_time <= now {
		(now + Duration::days(1))
			.date()
			.and_time(when)
			.ok_or(miette!("invalid time"))?
	} else {
		now_with_time
	};

	debug!(%starting, %duration, "recording sprint");
	let id: Uuid =
		sqlx::query("INSERT INTO sprints (starting_at, duration) VALUES ($1, $2) RETURNING id")
			.bind(starting)
			.bind(duration)
			.fetch_one(&app.db)
			.await
			.into_diagnostic()
			.wrap_err("storing to db")?
			.try_get("id")
			.into_diagnostic()
			.wrap_err("getting stored id")?;

	app.send_action(
		SprintAnnounce::new(app.clone(), &interaction, id)
			.await
			.wrap_err("rendering announce")?,
	)
	.await?;

	Ok(())
}

fn get_option<'o>(options: &'o [CommandDataOption], name: &str) -> Option<&'o CommandOptionValue> {
	options.iter().find_map(|opt| {
		if opt.name == name {
			Some(&opt.value)
		} else {
			None
		}
	})
}

fn get_string<'o>(options: &'o [CommandDataOption], name: &str) -> Option<&'o str> {
	get_option(options, name).and_then(|val| {
		if let CommandOptionValue::String(s) = val {
			Some(s.as_str())
		} else {
			None
		}
	})
}

fn get_integer<'o>(options: &'o [CommandDataOption], name: &str) -> Option<i64> {
	get_option(options, name).and_then(|val| {
		if let CommandOptionValue::Integer(i) = val {
			Some(*i)
		} else {
			None
		}
	})
}

fn parse_when_relative_to(now: NaiveTime, s: &str) -> Result<NaiveTime> {
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
