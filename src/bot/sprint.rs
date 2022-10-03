use std::str::FromStr;

use chrono::{Duration, Utc};
use miette::{miette, Context, IntoDiagnostic, Result};
use sqlx::{types::Uuid, Row};
use tracing::{debug, warn};
use twilight_model::application::{
	command::{Command, CommandType},
	interaction::{
		application_command::{CommandData, CommandDataOption, CommandOptionValue},
		message_component::MessageComponentInteractionData,
		Interaction,
	},
};
use twilight_util::builder::command::{
	CommandBuilder, IntegerBuilder, StringBuilder, SubCommandBuilder,
};

use crate::bot::{
	action::SprintAnnounce,
	parsers::{
		command::{get_integer, get_string},
		time::parse_when_relative_to,
	},
};

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

pub async fn on_command(
	app: App,
	interaction: &Interaction,
	command_data: &CommandData,
) -> Result<()> {
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

pub async fn on_component(
	app: App,
	interaction: &Interaction,
	subids: &[&str],
	component_data: &MessageComponentInteractionData,
) -> Result<()> {
	debug!(?subids, ?component_data, "sprint component action");

	match subids {
		["announce", "join", uuid] => sprint_join(app.clone(), interaction, *uuid)
			.await
			.wrap_err("action: announce:join")?,
		id => warn!(?id, "unhandled sprint component action"),
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

async fn sprint_join(app: App, interaction: &Interaction, uuid: &str) -> Result<()> {
	let uuid = Uuid::from_str(uuid).into_diagnostic()?;

	Ok(())
}
