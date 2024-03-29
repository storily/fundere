use miette::{miette, IntoDiagnostic, Result};
use tracing::debug;
use twilight_model::application::{
	command::{Command, CommandType},
	interaction::{application_command::CommandData, Interaction},
};
use twilight_util::builder::command::{BooleanBuilder, CommandBuilder, StringBuilder};

use crate::{
	bot::{
		action::{CalcResult, CommandAck},
		utils::command::{get_boolean, get_string},
		App,
	},
	error_ext::ErrorExt,
};

#[tracing::instrument]
pub fn command() -> Result<Command> {
	CommandBuilder::new(
		"calc",
		format!(
			"Calculate something! Uses fend {}",
			fend_core::get_version()
		),
		CommandType::ChatInput,
	)
	.option(StringBuilder::new("input", "What you want to calculate").required(true))
	.option(BooleanBuilder::new(
		"public",
		"Make the result public, instead of just for yourself",
	))
	.validate()
	.into_diagnostic()
	.map(|cmd| cmd.build())
}

pub async fn on_command(
	app: App,
	interaction: &Interaction,
	command_data: &CommandData,
) -> Result<()> {
	let public = get_boolean(&command_data.options, "public").unwrap_or(false);
	let input = get_string(&command_data.options, "input").ok_or(miette!("input is required"))?;
	app.do_action(if public {
		CommandAck::new(interaction)
	} else {
		CommandAck::ephemeral(interaction)
	})
	.await
	.log()
	.ok();
	debug!(?input, "calculating");

	let mut context = fend_core::Context::new();
	let result = fend_core::evaluate(input, &mut context).map_err(|err| miette!("{}", err))?;

	app.do_action(CalcResult::new(
		interaction,
		input,
		result.get_main_result(),
		public,
	)?)
	.await
}
