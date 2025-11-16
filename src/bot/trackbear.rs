use std::str::FromStr;

use miette::{miette, Context, IntoDiagnostic, Result};
use secret_vault_value::SecretValue;
use tracing::{debug, error, info, warn};
use twilight_model::application::{
	command::{Command, CommandType},
	interaction::{
		application_command::{CommandData, CommandDataOption, CommandOptionValue},
		message_component::MessageComponentInteractionData,
		modal::ModalInteractionData,
		Interaction,
	},
};
use twilight_util::builder::command::{CommandBuilder, SubCommandBuilder};
use uuid::Uuid;

use crate::{
	bot::{
		action::{CommandAck, ComponentAck, TrackbearLoginConfirm, TrackbearLoginModal},
		context::{GenericResponse, GenericResponseData},
		App,
	},
	db::{member::Member, trackbear_login::TrackbearLogin},
	error_ext::ErrorExt,
};

#[tracing::instrument]
pub fn command() -> Result<Command> {
	CommandBuilder::new(
		"trackbear",
		"TrackBear login control",
		CommandType::ChatInput,
	)
	.option(SubCommandBuilder::new("status", "Check your login status"))
	.option(SubCommandBuilder::new(
		"login",
		"Login to TrackBear with the bot",
	))
	.option(SubCommandBuilder::new(
		"logout",
		"Delete your TrackBear login from the bot",
	))
	.option(SubCommandBuilder::new(
		"projects",
		"List your TrackBear projects",
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
	let subcmd = command_data.options.iter().find_map(|opt| {
		if let CommandOptionValue::SubCommand(ref sub) = opt.value {
			Some((opt.name.as_str(), sub.as_slice()))
		} else {
			None
		}
	});

	match subcmd {
		Some(("status", opts)) => status(app.clone(), interaction, opts)
			.await
			.wrap_err("command: login")?,
		Some(("login", opts)) => login_confirm(app.clone(), interaction, opts)
			.await
			.wrap_err("command: login")?,
		Some(("logout", opts)) => logout(app.clone(), interaction, opts)
			.await
			.wrap_err("command: logout")?,
		Some(("projects", opts)) => list_projects(app.clone(), interaction, opts)
			.await
			.wrap_err("command: projects")?,
		Some((other, _)) => warn!("unhandled trackbear subcommand: {other}"),
		_ => error!("unreachable bare trackbear command"),
	}

	Ok(())
}

pub async fn on_component(
	app: App,
	interaction: &Interaction,
	subids: &[&str],
	component_data: &MessageComponentInteractionData,
) -> Result<()> {
	debug!(?subids, ?component_data, "trackbear component action");

	match subids {
		["login", uuid] => login_modal(app.clone(), interaction, uuid)
			.await
			.wrap_err("action: login")?,
		["set-project", project_id] => set_project(app.clone(), interaction, project_id)
			.await
			.wrap_err("action: set-project")?,
		id => warn!(?id, "unhandled trackbear component action"),
	}

	Ok(())
}

pub async fn on_modal(
	app: App,
	interaction: &Interaction,
	subids: &[&str],
	component_data: &ModalInteractionData,
) -> Result<()> {
	debug!(?subids, ?component_data, "trackbear modal action");

	match subids {
		["login", uuid] => login(app.clone(), interaction, uuid, component_data)
			.await
			.wrap_err("action: trackbear modal: login")?,
		id => warn!(?id, "unhandled trackbear modal action"),
	}

	Ok(())
}

async fn status(app: App, interaction: &Interaction, _options: &[CommandDataOption]) -> Result<()> {
	let member = Member::try_from(interaction)?;
	app.do_action(CommandAck::ephemeral(interaction))
		.await
		.log()
		.ok();
	app.send_response(GenericResponse::from_interaction(
		interaction,
		GenericResponseData {
			content: Some(
				if let Some(login) = TrackbearLogin::get_for_member(app.clone(), member).await? {
					debug!(?login.id, ?member, "checking trackbear credentials");
					if login.client().await.is_ok() {
						"🙌 You're logged in to TrackBear!".to_string()
					} else {
						"⁉️ I've got credentials for you but they're not working".to_string()
					}
				} else {
					"🙅 You're not logged in".to_string()
				},
			),
			ephemeral: true,
			..Default::default()
		},
	))
	.await
	.map(drop)
}

async fn login_confirm(
	app: App,
	interaction: &Interaction,
	_options: &[CommandDataOption],
) -> Result<()> {
	let member = Member::try_from(interaction)?;
	app.do_action(TrackbearLoginConfirm::new(interaction, member))
		.await
}

async fn login_modal(app: App, interaction: &Interaction, uuid: &str) -> Result<()> {
	let member: Member = Uuid::from_str(uuid).into_diagnostic()?.into();
	app.do_action(TrackbearLoginModal::new(interaction, member))
		.await
}

async fn login(
	app: App,
	interaction: &Interaction,
	uuid: &str,
	data: &ModalInteractionData,
) -> Result<()> {
	let member: Member = Uuid::from_str(uuid).into_diagnostic()?.into();

	let api_key = data
		.components
		.iter()
		.flat_map(|row| row.components.iter())
		.find_map(|component| {
			if component.custom_id == "api_key" {
				component.value.as_deref().map(SecretValue::from)
			} else {
				None
			}
		})
		.ok_or_else(|| miette!("API key is a required field"))?;

	app.do_action(ComponentAck::ephemeral(interaction))
		.await
		.log()
		.ok();

	let login = match TrackbearLogin::get_for_member(app.clone(), member).await? {
		Some(mut login) => {
			debug!(?login.id, "updating login");
			login.update(app.clone(), api_key).await?;
			info!(?login.id, ?member, "updated trackbear credentials");
			login
		}
		None => {
			debug!("creating login");
			let login = TrackbearLogin::create(app.clone(), member, api_key).await?;
			info!(?login.id, ?member, "recorded trackbear credentials");
			login
		}
	};

	debug!(?login.id, "checking trackbear credentials");
	let _ = login
		.client()
		.await
		.wrap_err("couldn't login to trackbear!")?;
	debug!(?login.id, "successfully logged into trackbear");

	app.send_response(GenericResponse::from_interaction(
		interaction,
		GenericResponseData {
			content: Some(format!("✔ You're logged in to TrackBear!\nYou can now show the wordcount of your projects and update your wordcount with this bot.")),
			ephemeral: true,
			..Default::default()
		},
	))
	.await
	.map(drop)
}

async fn logout(app: App, interaction: &Interaction, _options: &[CommandDataOption]) -> Result<()> {
	let member = Member::try_from(interaction)?;
	app.do_action(CommandAck::ephemeral(interaction))
		.await
		.log()
		.ok();
	app.send_response(GenericResponse::from_interaction(
		interaction,
		GenericResponseData {
			content: Some(
				if let Some(login) = TrackbearLogin::get_for_member(app.clone(), member).await? {
					debug!(?login.id, ?member, "deleting trackbear credentials");
					login.delete(app.clone()).await?;
					"👋 I've forgotten your TrackBear credentials!\nIf you want to check the wordcount of your projects or update your wordcount with this bot, you'll need to login again.".to_string()
				} else {
					"⁉️ You're not logged in to TrackBear with the bot".to_string()
				}
			),
			ephemeral: true,
			..Default::default()
		},
	))
	.await
	.map(drop)
}

async fn list_projects(
	app: App,
	interaction: &Interaction,
	_options: &[CommandDataOption],
) -> Result<()> {
	let member = Member::try_from(interaction)?;
	app.do_action(CommandAck::ephemeral(interaction))
		.await
		.log()
		.ok();

	let login = TrackbearLogin::get_for_member(app.clone(), member)
		.await?
		.ok_or_else(|| miette!("You need to /trackbear login first!"))?;

	let client = login.client().await?;
	let mut projects = client.list_projects().await?;

	// Sort by last updated (most recent first)
	projects.sort_by(|a, b| b.last_updated.cmp(&a.last_updated));

	// Take only the first 10
	projects.truncate(10);

	if projects.is_empty() {
		app.send_response(GenericResponse::from_interaction(
			interaction,
			GenericResponseData {
				content: Some("You don't have any projects in TrackBear yet!".to_string()),
				ephemeral: true,
				..Default::default()
			},
		))
		.await
		.map(drop)
	} else {
		use twilight_model::channel::message::component::{
			ActionRow, Button, ButtonStyle, Component,
		};

		let mut content = String::from("**Your TrackBear Projects:**\n\n");
		let mut components = Vec::new();

		for project in &projects {
			content.push_str(&format!(
				"• **{}** (ID: {}) - {} words\n",
				project.title,
				project.id,
				project.totals.word.unwrap_or_default()
			));

			components.push(Component::ActionRow(ActionRow {
				components: vec![Component::Button(Button {
					custom_id: Some(format!("trackbear:set-project:{}", project.id)),
					disabled: false,
					emoji: None,
					label: Some(format!("Set \"{}\" as current", project.title)),
					style: ButtonStyle::Primary,
					url: None,
				})],
			}));
		}

		app.send_response(GenericResponse::from_interaction(
			interaction,
			GenericResponseData {
				content: Some(content),
				components,
				ephemeral: true,
				..Default::default()
			},
		))
		.await
		.map(drop)
	}
}

async fn set_project(app: App, interaction: &Interaction, project_id_str: &str) -> Result<()> {
	let project_id = i64::from_str(project_id_str).into_diagnostic()?;
	let member = Member::try_from(interaction)?;

	app.do_action(ComponentAck::ephemeral(interaction))
		.await
		.log()
		.ok();

	let login = TrackbearLogin::get_for_member(app.clone(), member)
		.await?
		.ok_or_else(|| miette!("You need to /trackbear login first!"))?;

	let client = login.client().await?;

	// Verify the project exists and user has access
	let projects = client.list_projects().await?;
	let trackbear_project = projects
		.into_iter()
		.find(|p| p.id == project_id)
		.ok_or_else(|| {
			miette!(
				"Project with ID {} not found in your TrackBear account",
				project_id
			)
		})?;

	debug!(?trackbear_project, ?member, "saving project");

	use crate::db::project::Project;
	let project = Project::create_or_replace(app.clone(), member, project_id).await?;
	debug!(?project.id, ?member, "saved project");

	app.send_response(GenericResponse::from_interaction(
		interaction,
		GenericResponseData {
			content: Some(format!(
				"✅ Set **{}** as your current project! Use `/words show` to see your progress.",
				trackbear_project.title
			)),
			ephemeral: true,
			..Default::default()
		},
	))
	.await
	.map(drop)
}
