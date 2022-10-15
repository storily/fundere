use miette::{miette, Context, IntoDiagnostic, Result};
use tracing::{error, warn};
use twilight_model::application::{
	command::{Command, CommandType},
	interaction::{
		application_command::{CommandData, CommandDataOption, CommandOptionValue},
		Interaction,
	},
};
use twilight_util::builder::command::{CommandBuilder, SubCommandBuilder};

use super::App;

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

async fn throw_error(
	_app: App,
	_interaction: &Interaction,
	_options: &[CommandDataOption],
) -> Result<()> {
	Err(miette!("test error"))
}
