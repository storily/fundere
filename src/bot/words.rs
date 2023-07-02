use miette::{miette, Context, IntoDiagnostic, Result};
use tracing::{debug, error, warn};
use twilight_model::application::{
	command::{Command, CommandType},
	interaction::{
		application_command::{CommandData, CommandDataOption, CommandOptionValue},
		modal::ModalInteractionData,
		Interaction,
	},
};
use twilight_util::builder::command::{
	CommandBuilder, IntegerBuilder, StringBuilder, SubCommandBuilder,
};

use crate::bot::utils::command::get_string;

use super::App;

#[tracing::instrument]
pub fn command() -> Result<Command> {
	CommandBuilder::new("words", "Nanowrimo control", CommandType::ChatInput)
		.option(SubCommandBuilder::new(
			"show",
			"Show off your word count and any pretties",
		))
		.option(
			SubCommandBuilder::new("project", "Configure which project you're working on").option(
				StringBuilder::new("url", "The URL of the project in the nanowrimo site")
					.required(true),
			),
		)
		.option(
			SubCommandBuilder::new("goal", "Override your project's goal (useful in November)")
				.option(
					IntegerBuilder::new("words", "New goal in words, only applies to sassbot")
						.required(true),
				),
		)
		.option(
			SubCommandBuilder::new("record", "Set your word count")
				.option(IntegerBuilder::new("words", "New total word count").required(true)),
		)
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
		// Some(("show", opts)) => show(app.clone(), interaction, opts)
		// 	.await
		// 	.wrap_err("command: show")?,
		Some(("project", opts)) => project(app.clone(), interaction, opts)
			.await
			.wrap_err("command: project")?,
		// Some(("goal", opts)) => goal(app.clone(), interaction, opts)
		// 	.await
		// 	.wrap_err("command: goal")?,
		// Some(("record", opts)) => record(app.clone(), interaction, opts)
		// 	.await
		// 	.wrap_err("command: record")?,
		Some((other, _)) => warn!("unhandled words subcommand: {other}"),
		_ => error!("unreachable bare words command"),
	}

	Ok(())
}

pub async fn on_modal(
	_app: App,
	_interaction: &Interaction,
	subids: &[&str],
	component_data: &ModalInteractionData,
) -> Result<()> {
	debug!(?subids, ?component_data, "words modal action");

	match subids {
		id => warn!(?id, "unhandled words modal action"),
	}

	Ok(())
}

async fn project(
	_app: App,
	_interaction: &Interaction,
	options: &[CommandDataOption],
) -> Result<()> {
	let _url = get_string(options, "url").ok_or_else(|| miette!("missing url"))?;

	Ok(())
}
