use miette::{miette, Context, IntoDiagnostic, Result};
use nanowrimo::{NanoKind, ObjectInfo, ProjectObject, ItemResponse};
use tracing::{debug, error, warn};
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

use crate::{
	bot::{
		context::{GenericResponse, GenericResponseData},
		utils::command::get_string,
	},
	db::{member::Member, nanowrimo_login::NanowrimoLogin, project::Project},
};

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
		Some(("project", opts)) => set_project(app.clone(), interaction, opts)
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

async fn set_project(
	app: App,
	interaction: &Interaction,
	options: &[CommandDataOption],
) -> Result<()> {
	let input = get_string(options, "url").ok_or_else(|| miette!("missing url"))?;
	let slug = if input.starts_with("https:") {
		input
			.split('/')
			.last()
			.ok_or_else(|| miette!("malformed url"))?
	} else {
		input
	};

	let member = Member::try_from(interaction)?;
	let client = NanowrimoLogin::client_for_member_or_default(app.clone(), member).await?;

	debug!(?slug, ?member, "checking project exists / is accessible");
	let nano_project: ItemResponse<ProjectObject> = client
		.get_slug(NanoKind::Project, slug)
		.await
		.into_diagnostic()?;

	debug!(?nano_project, ?member, "saving project");
	let project = Project::create_or_replace(app.clone(), member, nano_project.data.id()).await?;
	debug!(?nano_project, ?project.id, ?member, "saved project");

	app.send_response(GenericResponse::from_interaction(
		interaction,
		GenericResponseData {
			content: Some(format!("Got it! To show off your wordcount for {}, call **/words show**", nano_project.data.attributes.title)),
			ephemeral: true,
			..Default::default()
		},
	))
	.await
	.map(drop)
}
