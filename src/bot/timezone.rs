use std::str::FromStr;

use miette::{miette, Context, IntoDiagnostic, Result};
use tracing::{debug, info, warn};
use twilight_model::application::{
	command::{Command, CommandType},
	interaction::{
		application_command::CommandData, message_component::MessageComponentInteractionData,
		modal::ModalInteractionData, Interaction,
	},
};
use twilight_util::builder::command::CommandBuilder;
use uuid::Uuid;

use crate::{
	bot::{
		action::{CommandAck, ComponentAck, TimezoneModal, TimezoneShow},
		context::{GenericResponse, GenericResponseData},
		App,
	},
	db::{member::Member, user_preference::UserPreference},
	error_ext::ErrorExt,
};

#[tracing::instrument]
pub fn command() -> Result<Command> {
	CommandBuilder::new(
		"timezone",
		"View and set your timezone for time-based commands",
		CommandType::ChatInput,
	)
	.validate()
	.into_diagnostic()
	.map(|cmd| cmd.build())
}

pub async fn on_command(
	app: App,
	interaction: &Interaction,
	_command_data: &CommandData,
) -> Result<()> {
	let member = Member::try_from(interaction)?;
	app.do_action(CommandAck::ephemeral(interaction))
		.await
		.log()
		.ok();

	let prefs = UserPreference::get_or_create(app.clone(), member).await?;

	app.do_action(TimezoneShow::new(interaction, member, prefs.timezone))
		.await
}

pub async fn on_component(
	app: App,
	interaction: &Interaction,
	subids: &[&str],
	component_data: &MessageComponentInteractionData,
) -> Result<()> {
	debug!(?subids, ?component_data, "timezone component action");

	match subids {
		["change", uuid] => timezone_change(app.clone(), interaction, uuid)
			.await
			.wrap_err("action: change")?,
		id => warn!(?id, "unhandled timezone component action"),
	}

	Ok(())
}

pub async fn on_modal(
	app: App,
	interaction: &Interaction,
	subids: &[&str],
	component_data: &ModalInteractionData,
) -> Result<()> {
	debug!(?subids, ?component_data, "timezone modal action");

	match subids {
		["set", uuid] => timezone_set(app.clone(), interaction, uuid, component_data)
			.await
			.wrap_err("action: timezone modal: set")?,
		id => warn!(?id, "unhandled timezone modal action"),
	}

	Ok(())
}

async fn timezone_change(app: App, interaction: &Interaction, uuid: &str) -> Result<()> {
	let member: Member = Uuid::from_str(uuid).into_diagnostic()?.into();
	let prefs = UserPreference::get_or_create(app.clone(), member).await?;

	app.do_action(TimezoneModal::new(interaction, member, prefs.timezone))
		.await
}

async fn timezone_set(
	app: App,
	interaction: &Interaction,
	uuid: &str,
	data: &ModalInteractionData,
) -> Result<()> {
	let member: Member = Uuid::from_str(uuid).into_diagnostic()?.into();

	let timezone_str = data
		.components
		.iter()
		.flat_map(|row| row.components.iter())
		.find_map(|component| {
			if component.custom_id == "timezone" {
				component.value.as_deref()
			} else {
				None
			}
		})
		.ok_or_else(|| miette!("timezone is a required field"))?;

	// Validate that the timezone is valid
	let _tz: chrono_tz::Tz = timezone_str
		.parse()
		.map_err(|_| miette!("Invalid timezone. Please use IANA timezone format (e.g., America/New_York, Europe/London, Pacific/Auckland)"))?;

	app.do_action(ComponentAck::ephemeral(interaction))
		.await
		.log()
		.ok();

	let prefs = UserPreference::get_or_create(app.clone(), member).await?;
	prefs
		.set_timezone(app.clone(), timezone_str.to_string())
		.await?;

	info!(?member, %timezone_str, "updated user timezone");

	app.send_response(GenericResponse::default())
		.await
		.map(drop)
}
