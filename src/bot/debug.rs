use std::str::FromStr;

use miette::{miette, Context, IntoDiagnostic, Result};
use tracing::{debug, error, warn};
use twilight_mention::Mention;
use twilight_model::{
	application::{
		command::{Command, CommandType},
		interaction::{
			application_command::{CommandData, CommandDataOption, CommandOptionValue},
			message_component::MessageComponentInteractionData,
			Interaction,
		},
	},
	id::Id,
};
use twilight_util::builder::{
	command::{CommandBuilder, SubCommandBuilder},
	embed::EmbedBuilder,
};
use uuid::Uuid;

use crate::{
	bot::{action::ComponentAck, App},
	db::error::Error,
	error_ext::ErrorExt,
};

#[tracing::instrument]
pub fn command() -> Result<Command> {
	CommandBuilder::new(
		"debug",
		"Debugging utilities and commands",
		CommandType::ChatInput,
	)
	.option(SubCommandBuilder::new("error", "Throw an error"))
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
		Some(("error", opts)) => throw_error(app.clone(), interaction, opts)
			.await
			.wrap_err("command: error")?,
		Some((other, _)) => warn!("unhandled debug subcommand: {other}"),
		_ => error!("unreachable bare debug command"),
	}

	Ok(())
}

pub async fn on_component(
	app: App,
	interaction: &Interaction,
	subids: &[&str],
	component_data: &MessageComponentInteractionData,
) -> Result<()> {
	debug!(?subids, ?component_data, "debug component action");

	match subids {
		["publish-error", uuid] => ping_maintainer(app.clone(), interaction, *uuid)
			.await
			.wrap_err("action: publish-error")?,
		id => warn!(?id, "unhandled debug component action"),
	}

	Ok(())
}

async fn throw_error(
	_app: App,
	_interaction: &Interaction,
	_options: &[CommandDataOption],
) -> Result<()> {
	Err(miette!("test error"))
}

async fn ping_maintainer(app: App, interaction: &Interaction, uuid: &str) -> Result<()> {
	app.do_action(ComponentAck::ephemeral(&interaction))
		.await
		.log()
		.ok();

	let maintainer = Id::new(match app.config.discord.maintainer_id {
		Some(id) => id,
		None => {
			warn!("no maintainer-id, but got a publish-error action");
			return Ok(());
		}
	});

	let uuid = Uuid::from_str(uuid).into_diagnostic()?;
	let error = Error::get(app.clone(), uuid).await?;

	if error.reported {
		warn!("error already reported");
		return Ok(());
	}

	let dm_channel = app
		.client
		.create_private_channel(maintainer)
		.await
		.into_diagnostic()?
		.model()
		.await
		.into_diagnostic()?;

	app.client
		.create_message(dm_channel.id)
		.embeds(&[EmbedBuilder::new()
			.color(0xFF_00_00)
			.description(error.message.clone())
			.validate()
			.into_diagnostic()?
			.build()])
		.into_diagnostic()?
		.content(&format!(
			"Error from {}, occurred at {}",
			error.member.mention(),
			error.created_at
		))
		.into_diagnostic()?
		.await
		.into_diagnostic()?;

	error.set_reported(app.clone()).await
}
