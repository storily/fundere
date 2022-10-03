use std::str::FromStr;

use chrono::{Duration, Utc};
use miette::{miette, Context, IntoDiagnostic, Result};
use sqlx::types::Uuid;
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

use crate::{
	bot::{
		action::{SprintAnnounce, SprintJoined},
		utils::{
			command::{get_integer, get_string},
			time::parse_when_relative_to,
		},
	},
	db::sprint::{Sprint, SprintStatus},
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
		["announce", "cancel", uuid] => sprint_cancel(app.clone(), interaction, *uuid)
			.await
			.wrap_err("action: announce:cancel")?,
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
	let id = Sprint::create(app.clone(), starting, duration).await?;
	let sprint = Sprint::from_current(app.clone(), id)
		.await
		.wrap_err("BUG: new sprint isn't current!")?;

	app.send_action(
		SprintAnnounce::new(app.clone(), &interaction, sprint)
			.await
			.wrap_err("rendering announce")?,
	)
	.await?;

	Ok(())
}

async fn sprint_join(app: App, interaction: &Interaction, uuid: &str) -> Result<()> {
	let uuid = Uuid::from_str(uuid).into_diagnostic()?;
	let sprint = Sprint::from_current(app.clone(), uuid)
		.await
		.wrap_err("that sprint isn't current")?;

	let guild_id = interaction
		.guild_id
		.ok_or(miette!("can only join sprint from a guild"))?;
	let user = interaction
		.member
		.as_ref()
		.and_then(|m| m.user.as_ref())
		.ok_or(miette!("can only join sprint from a guild"))?;

	if sprint.status()? >= SprintStatus::Ended {
		return Err(miette!("sprint has already ended"));
	}

	sprint.join(app.clone(), guild_id, user.id).await?;

	app.send_action(SprintJoined::new(&interaction, sprint))
		.await?;

	Ok(())
}

async fn sprint_cancel(app: App, interaction: &Interaction, uuid: &str) -> Result<()> {
	let uuid = Uuid::from_str(uuid).into_diagnostic()?;
	let sprint = Sprint::from_current(app.clone(), uuid)
		.await
		.wrap_err("that sprint isn't current")?;

	Ok(())
}
