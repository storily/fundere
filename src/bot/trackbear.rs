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
	let client = login
		.client()
		.await
		.wrap_err("couldn't login to trackbear!")?;
	debug!(?login.id, "successfully logged into trackbear");

	// TODO: Get username from TrackBear API
	let name = "TrackBear User";

	app.send_response(GenericResponse::from_interaction(
		interaction,
		GenericResponseData {
			content: Some(format!("✔ You're logged in to TrackBear as {name}!\nYou can now show the wordcount of your projects and update your wordcount with this bot.")),
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
