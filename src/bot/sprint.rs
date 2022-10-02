use std::iter::repeat;

use miette::{Context, GraphicalReportHandler, GraphicalTheme, IntoDiagnostic, Result};
use tracing::{info, warn};
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

use super::{
	action::{Action, CommandError},
	App,
};

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

	if let Err(err) = match subcmd {
		Some(("start", opts)) => sprint_start(app.clone(), interaction, opts)
			.await
			.wrap_err("command: start"),
		Some((other, _)) => {
			warn!("unhandled sprint subcommand: {other}");
			Ok(())
		}
		_ => todo!("handle bare sprint command?"),
	} {
		let handler = GraphicalReportHandler::new_themed(GraphicalTheme::unicode_nocolor());
		let mut error = String::from("Error:\n```");
		handler
			.render_report(&mut error, err.as_ref())
			.into_diagnostic()?;
		error.extend(repeat('`').take(3));
		app.send_action(Action::CommandError(CommandError {
			id: interaction.id,
			token: interaction.token.clone(),
			error,
		}))
		.await?;
	}

	Ok(())
}

async fn sprint_start(
	_app: App,
	_interaction: &Interaction,
	options: &[CommandDataOption],
) -> Result<()> {
	let when = get_string(options, "when").unwrap_or("15m");
	let duration = get_integer(options, "duration").unwrap_or(15);

	info!(?when, ?duration, "start sprint options");

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
