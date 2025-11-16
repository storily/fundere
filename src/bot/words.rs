use std::str::FromStr;

use miette::{miette, Context, IntoDiagnostic, Result};
use nanowrimo::{ItemResponse, NanoKind, ObjectInfo, ProjectObject};
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
		action::CommandAck,
		context::{GenericResponse, GenericResponseData},
		utils::command::{get_integer, get_string},
		App,
	},
	db::{member::Member, project::Project, trackbear_login::TrackbearLogin},
	error_ext::ErrorExt,
	nano::project::Project as NanoProject,
};

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
			SubCommandBuilder::new(
				"goal",
				"Override your project's goal (useful in November). 0 unsets the custom goal.",
			)
			.option(
				IntegerBuilder::new("words", "New goal in words, only applies to sassbot")
					.required(true),
			),
		)
		.option(
			SubCommandBuilder::new("record", "Set your word count").option(
				StringBuilder::new(
					"words",
					"New total word count, or relative using ++/-- prefixes",
				)
				.required(true),
			),
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
		Some(("show", _opts)) => show(app.clone(), interaction)
			.await
			.wrap_err("command: show")?,
		Some(("project", opts)) => set_project(app.clone(), interaction, opts)
			.await
			.wrap_err("command: project")?,
		Some(("goal", opts)) => override_goal(app.clone(), interaction, opts)
			.await
			.wrap_err("command: goal")?,
		Some(("record", opts)) => record_words(app.clone(), interaction, opts)
			.await
			.wrap_err("command: record")?,
		Some((other, _)) => warn!("unhandled words subcommand: {other}"),
		_ => error!("unreachable bare words command"),
	}

	Ok(())
}

async fn show(app: App, interaction: &Interaction) -> Result<()> {
	let member = Member::try_from(interaction)?;
	app.do_action(CommandAck::new(interaction)).await.log().ok();
	let project = Project::get_for_member(app.clone(), member)
		.await?
		.ok_or_else(|| miette!("no project set up! Use /words project"))?;
	show_followup(app, interaction, &project).await
}

async fn show_followup(app: App, interaction: &Interaction, project: &Project) -> Result<()> {
	let text = project.show_text(app.clone()).await?;
	debug!(?project, ?text, "about to show this");

	app.send_response(GenericResponse::from_interaction(
		interaction,
		GenericResponseData {
			content: Some(text),
			..Default::default()
		},
	))
	.await
	.map(drop)
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
	app.do_action(CommandAck::ephemeral(interaction))
		.await
		.log()
		.ok();

	let member = Member::try_from(interaction)?;
	let client = TrackbearLogin::client_for_member(app.clone(), member).await?;

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
			content: Some(format!(
				"Got it! To show off your wordcount for {}, call **/words show**",
				nano_project.data.attributes.title
			)),
			ephemeral: true,
			..Default::default()
		},
	))
	.await?;

	show_followup(app, interaction, &project).await
}

async fn override_goal(
	app: App,
	interaction: &Interaction,
	options: &[CommandDataOption],
) -> Result<()> {
	let goal = get_integer(options, "words").ok_or_else(|| miette!("missing goal in words"))?;

	let member = Member::try_from(interaction)?;
	app.do_action(CommandAck::ephemeral(interaction))
		.await
		.log()
		.ok();

	let project = Project::get_for_member(app.clone(), member)
		.await?
		.ok_or_else(|| miette!("no project set up! Use /words project"))?;

	debug!(?project.id, ?goal, "updating project");
	let content = if goal > 0 {
		project.set_goal(app.clone(), goal as _).await?;
		debug!(?project.id, ?goal, "set custom goal");
		format!("Got it! Your new sassbot-only goal is **{goal}**.",)
	} else {
		project.unset_goal(app.clone()).await?;
		debug!(?project.id, "unset custom goal");
		"Your goal has been reverted to the one from the nano website (if any).".to_string()
	};

	app.send_response(GenericResponse::from_interaction(
		interaction,
		GenericResponseData {
			content: Some(content),
			ephemeral: true,
			..Default::default()
		},
	))
	.await?;

	show_followup(app, interaction, &project).await
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SaveWords {
	Absolute(u64),
	Relative(i64),
}

async fn record_words(
	app: App,
	interaction: &Interaction,
	options: &[CommandDataOption],
) -> Result<()> {
	let words = get_string(options, "words")
		.ok_or_else(|| miette!("missing words"))
		.and_then(|input| {
			debug!(?input, "words record: raw input");
			if input.starts_with('+') {
				i64::from_str(input.trim_start_matches('+')).map(SaveWords::Relative)
			} else if input.starts_with('-') {
				i64::from_str(input.trim_start_matches('-')).map(|n| SaveWords::Relative(-n))
			} else {
				u64::from_str(input).map(SaveWords::Absolute)
			}
			.into_diagnostic()
		})?;
	debug!(?words, "words record: parsed input");

	let member = Member::try_from(interaction)?;
	app.do_action(CommandAck::ephemeral(interaction))
		.await
		.log()
		.ok();

	let project = Project::get_for_member(app.clone(), member)
		.await?
		.ok_or_else(|| miette!("no project set up! Use /words project"))?;
	let login = TrackbearLogin::get_for_member(app.clone(), member)
		.await?
		.ok_or_else(|| miette!("You need to /trackbear login to be able to record words!"))?;

	save_words(app, interaction, &login, &project, words).await
}

pub async fn save_words(
	app: App,
	interaction: &Interaction,
	login: &TrackbearLogin,
	project: &Project,
	words: SaveWords,
) -> Result<()> {
	let client = login.client().await?;
	let nano_project = NanoProject::fetch_with_client(client.clone(), project.nano_id).await?;
	let Some(goal) = nano_project.current_goal() else {
		return Err(miette!("no goal set up on the nano site"));
	};

	let session_word_count = match words {
		SaveWords::Absolute(n) => n.saturating_sub(nano_project.wordcount()) as i64,
		SaveWords::Relative(n) => n,
	};

	debug!(?project.id, ?nano_project, ?session_word_count, "posting new wordcount session to nano");
	let saved_session = client
		.add_project_session(nano_project.id, goal.id, session_word_count)
		.await
		.into_diagnostic()
		.wrap_err("nano: failed to update wordcount")?;
	debug!(?project.id, ?nano_project, ?session_word_count, ?saved_session, "created wordcount session on nano");

	app.send_response(GenericResponse::from_interaction(
		interaction,
		GenericResponseData {
			content: Some("Updated your word count on nanowrimo.org".to_string()),
			ephemeral: true,
			..Default::default()
		},
	))
	.await?;

	show_followup(app, interaction, project).await
}
