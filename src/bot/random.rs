use itertools::Itertools;
use miette::{miette, IntoDiagnostic, Result, Context};
use rand::{distributions::Uniform, Rng};
use rand::seq::SliceRandom;
use regex::Regex;
use tracing::{debug, warn, error};
use twilight_model::application::{
	command::{Command, CommandType},
	interaction::{application_command::{CommandData, CommandDataOption, CommandOptionValue}, Interaction},
};
use twilight_util::builder::command::{
	CommandBuilder, IntegerBuilder, StringBuilder, SubCommandBuilder,
};

use crate::bot::{
	context::{GenericResponse, GenericResponseData},
	utils::command::{get_integer, get_string},
};

use super::App;


#[tracing::instrument]
pub fn command() -> Result<Command> {
	CommandBuilder::new(
		"random",
		"Get some random in your life".to_string(),
		CommandType::ChatInput,
	)
	.option(
		SubCommandBuilder::new("number", "Get a random number")
			.option(IntegerBuilder::new(
				"min",
				"Minimum value (default: 0)",
			))
			.option(IntegerBuilder::new(
				"max",
				"Maximum value (default: as high as we can go)",
			))
			.option(IntegerBuilder::new(
				"count",
				"How many numbers to get (default: 1)",
			)),
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
		Some(("number", opts)) => number(app.clone(), interaction, opts)
			.await
			.wrap_err("command: number")?,
		Some((other, _)) => warn!("unhandled random subcommand: {other}"),
		_ => error!("unreachable bare random command"),
	}

	Ok(())
}

async fn number(
	app: App,
	interaction: &Interaction,
	options: &[CommandDataOption],
) -> Result<()> {
	let count = get_integer(options, "count").unwrap_or(1);
	let min = get_integer(options, "min").unwrap_or(0);
	let max = get_integer(options, "max").unwrap_or(i64::MAX);

	let result = (0..count)
		.map(|_| rand::thread_rng().gen_range(min..=max))
		.map(|n| format!("**{}**", n))
		.join(", ");

	app.send_response(GenericResponse::from_interaction(
		interaction,
		GenericResponseData {
			content: Some(result),
			..Default::default()
		},
	))
	.await
	.map(drop)
}

