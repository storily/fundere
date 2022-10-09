use std::str::FromStr;

use chrono::{Duration, Utc};
use miette::{miette, Context, IntoDiagnostic, Result};
use tracing::{debug, warn};
use twilight_model::application::{
	command::{Command, CommandType},
	interaction::{
		application_command::{CommandData, CommandDataOption, CommandOptionValue},
		message_component::MessageComponentInteractionData,
		modal::ModalInteractionData,
		Interaction,
	},
};
use twilight_util::builder::command::{
	CommandBuilder, IntegerBuilder, StringBuilder, SubCommandBuilder,
};
use uuid::Uuid;

use crate::{
	bot::{
		action::{SprintAnnounce, SprintCancelled, SprintJoined},
		utils::{
			command::{get_integer, get_string},
			time::parse_when_relative_to,
		},
	},
	db::{
		sprint::{Sprint, SprintStatus},
		types::Member,
	},
};

use super::{
	action::{CommandAck, SprintLeft, SprintWordsEnd, SprintWordsStart},
	App,
};

#[tracing::instrument]
pub fn command() -> Result<Command> {
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
		["join", uuid] => sprint_join(app.clone(), interaction, *uuid)
			.await
			.wrap_err("action: join")?,
		["leave", uuid] => sprint_leave(app.clone(), interaction, *uuid)
			.await
			.wrap_err("action: leave")?,
		["cancel", uuid] => sprint_cancel(app.clone(), interaction, *uuid)
			.await
			.wrap_err("action: cancel")?,
		["start-words", uuid] => sprint_words_start(app.clone(), interaction, *uuid)
			.await
			.wrap_err("action: words modal: start")?,
		["end-words", uuid] => sprint_words_end(app.clone(), interaction, *uuid)
			.await
			.wrap_err("action: words modal: end")?,
		id => warn!(?id, "unhandled sprint component action"),
	}

	Ok(())
}

pub async fn on_modal(
	app: App,
	interaction: &Interaction,
	subids: &[&str],
	component_data: &ModalInteractionData,
) -> Result<()> {
	debug!(?subids, ?component_data, "sprint modal action");

	match subids {
		["set-words", "start", uuid] => sprint_set_words(
			app.clone(),
			interaction,
			*uuid,
			component_data,
			"words_start",
		)
		.await
		.wrap_err("action: words modal: starting")?,
		["set-words", "end", uuid] => {
			sprint_set_words(app.clone(), interaction, *uuid, component_data, "words_end")
				.await
				.wrap_err("action: words modal: ending")?
		}
		id => warn!(?id, "unhandled sprint modal action"),
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
	let starting = if now_with_time < now {
		(now + Duration::days(1))
			.date()
			.and_time(when)
			.ok_or(miette!("invalid time"))?
	} else {
		now_with_time
	};

	let member = Member::try_from(interaction)?;

	debug!(%starting, %duration, ?member, "recording sprint");
	let sprint = Sprint::create(app.clone(), starting, duration, member).await?;

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
	let member = Member::try_from(interaction)?;
	let sprint = Sprint::get_current(app.clone(), uuid)
		.await
		.wrap_err("that sprint isn't current")?;

	if sprint.status >= SprintStatus::Ended {
		return Err(miette!("sprint has already ended"));
	}

	sprint.join(app.clone(), member).await?;

	app.send_action(SprintJoined::new(&interaction, sprint))
		.await?;

	Ok(())
}

async fn sprint_leave(app: App, interaction: &Interaction, uuid: &str) -> Result<()> {
	let uuid = Uuid::from_str(uuid).into_diagnostic()?;

	let member = Member::try_from(interaction)?;
	let sprint = Sprint::get_current(app.clone(), uuid)
		.await
		.wrap_err("that sprint isn't current")?;

	if sprint.status >= SprintStatus::Ended {
		return Err(miette!("sprint has already ended"));
	}

	sprint.leave(app.clone(), member).await?;

	app.send_action(SprintLeft::new(&interaction, &sprint))
		.await?;

	if sprint.participants(app.clone()).await?.is_empty() {
		let user = interaction
			.member
			.as_ref()
			.and_then(|m| m.user.as_ref())
			.ok_or(miette!("can only leave sprint from a guild"))?;

		app.send_action(SprintCancelled::new(&interaction, sprint.shortid, user).as_followup())
			.await?;
	}

	Ok(())
}

async fn sprint_cancel(app: App, interaction: &Interaction, uuid: &str) -> Result<()> {
	let uuid = Uuid::from_str(uuid).into_diagnostic()?;
	let sprint = Sprint::get_current(app.clone(), uuid)
		.await
		.wrap_err("that sprint isn't current")?;

	let user = interaction
		.member
		.as_ref()
		.and_then(|m| m.user.as_ref())
		.ok_or(miette!("can only cancel sprint from a guild"))?;

	if sprint.status >= SprintStatus::Ended {
		return Err(miette!("sprint has already ended"));
	}

	sprint.cancel(app.clone()).await?;

	app.send_action(SprintCancelled::new(&interaction, sprint.shortid, user))
		.await?;

	Ok(())
}

async fn sprint_words_start(app: App, interaction: &Interaction, uuid: &str) -> Result<()> {
	let uuid = Uuid::from_str(uuid).into_diagnostic()?;
	let sprint = Sprint::get(app.clone(), uuid)
		.await
		.wrap_err("sprint not found")?;

	if sprint.is_cancelled() {
		return Err(miette!("sprint was cancelled"));
	}
	if sprint.status >= SprintStatus::Summaried {
		return Err(miette!("sprint has already been finalised"));
	}

	app.send_action(SprintWordsStart::new(&interaction, sprint.id))
		.await?;

	Ok(())
}

async fn sprint_words_end(app: App, interaction: &Interaction, uuid: &str) -> Result<()> {
	let uuid = Uuid::from_str(uuid).into_diagnostic()?;
	let member = Member::try_from(interaction)?;
	let sprint = Sprint::get(app.clone(), uuid)
		.await
		.wrap_err("sprint not found")?;

	if sprint.is_cancelled() {
		return Err(miette!("sprint was cancelled"));
	}
	if sprint.status >= SprintStatus::Summaried {
		return Err(miette!("sprint has already been finalised"));
	}

	app.send_action(SprintWordsEnd::new(&interaction, sprint.id, member))
		.await?;

	Ok(())
}

async fn sprint_set_words(
	app: App,
	interaction: &Interaction,
	uuid: &str,
	data: &ModalInteractionData,
	column: &str,
) -> Result<()> {
	let uuid = Uuid::from_str(uuid).into_diagnostic()?;
	let member = Member::try_from(interaction)?;
	let sprint = Sprint::get(app.clone(), uuid)
		.await
		.wrap_err("sprint not found")?;

	if sprint.is_cancelled() {
		return Err(miette!("sprint was cancelled"));
	}
	if sprint.status >= SprintStatus::Summaried {
		return Err(miette!("sprint has already been finalised"));
	}

	let words = data
		.components
		.iter()
		.flat_map(|row| row.components.iter())
		.find_map(|component| {
			if component.custom_id == "words" {
				Some(&component.value)
			} else {
				None
			}
		})
		.ok_or_else(|| miette!("words is a required field"))?
		.as_deref()
		.map(|words| i32::from_str(words).into_diagnostic())
		.transpose()?
		.unwrap_or(0);

	sprint.set_words(app.clone(), member, words, column).await?;

	app.send_action(CommandAck::new(&interaction)).await?;

	Ok(())
}
