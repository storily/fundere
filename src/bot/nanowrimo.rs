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
		action::{CommandAck, ComponentAck, NanowrimoLoginConfirm, NanowrimoLoginModal},
		context::{GenericResponse, GenericResponseData},
	},
	db::{member::Member, nanowrimo_login::NanowrimoLogin},
};

use super::App;

#[tracing::instrument]
pub fn command() -> Result<Command> {
	CommandBuilder::new(
		"nanowrimo",
		"Nanowrimo login control",
		CommandType::ChatInput,
	)
	.option(SubCommandBuilder::new("status", "Check your login status"))
	.option(SubCommandBuilder::new(
		"login",
		"Login to nanowrimo with the bot",
	))
	.option(SubCommandBuilder::new(
		"logout",
		"Delete your nanowrimo login from the bot",
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
		Some((other, _)) => warn!("unhandled nanowrimo subcommand: {other}"),
		_ => error!("unreachable bare nanowrimo command"),
	}

	Ok(())
}

pub async fn on_component(
	app: App,
	interaction: &Interaction,
	subids: &[&str],
	component_data: &MessageComponentInteractionData,
) -> Result<()> {
	debug!(?subids, ?component_data, "nanowrimo component action");

	match subids {
		["login", uuid] => login_modal(app.clone(), interaction, *uuid)
			.await
			.wrap_err("action: login")?,
		id => warn!(?id, "unhandled nanowrimo component action"),
	}

	Ok(())
}

pub async fn on_modal(
	app: App,
	interaction: &Interaction,
	subids: &[&str],
	component_data: &ModalInteractionData,
) -> Result<()> {
	debug!(?subids, ?component_data, "nanowrimo modal action");

	match subids {
		["login", uuid] => login(app.clone(), interaction, *uuid, component_data)
			.await
			.wrap_err("action: nanowrimo modal: login")?,
		id => warn!(?id, "unhandled nanowrimo modal action"),
	}

	Ok(())
}

async fn status(app: App, interaction: &Interaction, _options: &[CommandDataOption]) -> Result<()> {
	let member = Member::try_from(interaction)?;
	app.do_action(CommandAck::ephemeral(&interaction)).await?;
	app.send_response(GenericResponse::from_interaction(
		interaction,
		GenericResponseData {
			content: Some(
				if let Some(login) = NanowrimoLogin::get_for_member(app.clone(), member).await? {
					debug!(?login.id, ?member, "checking nano credentials");
					if let Ok(client) = login.client().await {
						let nano_user = client.current_user().await.into_diagnostic()?.data;
						format!("🙌 You're logged in as {}", nano_user.attributes.name)
					} else {
						format!("⁉️ I've got credentials for you but they're not working")
					}
				} else {
					format!("🙅 You're not logged in")
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
	app.do_action(NanowrimoLoginConfirm::new(&interaction, member))
		.await
}

async fn login_modal(app: App, interaction: &Interaction, uuid: &str) -> Result<()> {
	let member: Member = Uuid::from_str(uuid).into_diagnostic()?.into();
	app.do_action(NanowrimoLoginModal::new(&interaction, member))
		.await
}

async fn login(
	app: App,
	interaction: &Interaction,
	uuid: &str,
	data: &ModalInteractionData,
) -> Result<()> {
	let member: Member = Uuid::from_str(uuid).into_diagnostic()?.into();

	let username = data
		.components
		.iter()
		.flat_map(|row| row.components.iter())
		.find_map(|component| {
			if component.custom_id == "username" {
				component.value.as_deref()
			} else {
				None
			}
		})
		.ok_or_else(|| miette!("username is a required field"))?;

	let password = data
		.components
		.iter()
		.flat_map(|row| row.components.iter())
		.find_map(|component| {
			if component.custom_id == "password" {
				component.value.as_deref().map(SecretValue::from)
			} else {
				None
			}
		})
		.ok_or_else(|| miette!("password is a required field"))?;

	app.do_action(ComponentAck::ephemeral(&interaction)).await?;

	let login = match NanowrimoLogin::get_for_member(app.clone(), member).await? {
		Some(mut login) => {
			debug!(?login.id, "updating login");
			login.update(app.clone(), &username, password).await?;
			info!(?login.id, ?member, "updated nanowrimo credentials");
			login
		}
		None => {
			debug!("creating login");
			let login = NanowrimoLogin::create(app.clone(), member, &username, password).await?;
			info!(?login.id, ?member, "recorded nanowrimo credentials");
			login
		}
	};

	debug!(?login.id, "checking nano credentials");
	let client = login
		.client()
		.await
		.wrap_err("couldn't login to nanowrimo!")?;
	debug!(?login.id, "successfully logged into nano");

	let name = client
		.current_user()
		.await
		.into_diagnostic()?
		.data
		.attributes
		.name;

	app.send_response(GenericResponse::from_interaction(
		interaction,
		GenericResponseData {
			content: Some(format!("✔ You're logged in to nanowrimo as {name}!\nYou can now show the wordcount of private projects and update your wordcount with this bot.")),
			ephemeral: true,
			..Default::default()
		},
	))
	.await
	.map(drop)
}

async fn logout(app: App, interaction: &Interaction, _options: &[CommandDataOption]) -> Result<()> {
	let member = Member::try_from(interaction)?;
	app.do_action(CommandAck::ephemeral(&interaction)).await?;
	app.send_response(GenericResponse::from_interaction(
		interaction,
		GenericResponseData {
			content: Some(
				if let Some(login) = NanowrimoLogin::get_for_member(app.clone(), member).await? {
					debug!(?login.id, ?member, "deleting nano credentials");
					login.delete(app.clone()).await?;
					format!("👋 I've forgotten your nanowrimo credentials!\nIf you want to check the wordcount of private projects or update your wordcount with this bot, you'll need to login again.")
				} else {
					format!("⁉️ You're not logged in to nanowrimo with the bot")
				}
			),
			ephemeral: true,
			..Default::default()
		},
	))
	.await
	.map(drop)
}
