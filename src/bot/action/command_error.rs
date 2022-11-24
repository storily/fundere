use std::iter::repeat;

use miette::{GraphicalReportHandler, GraphicalTheme, IntoDiagnostic, Report, Result};
use twilight_model::{
	channel::message::component::{ButtonStyle, Button, Component},
	application::{
	interaction::Interaction,
}};
use twilight_util::builder::embed::EmbedBuilder;

use crate::{
	bot::{
		context::{GenericResponse, GenericResponseData},
		utils::action_row,
		App,
	},
	db::{error::Error, member::Member},
};

use super::{Action, ActionClass, Args};

#[derive(Debug, Clone)]
pub struct CommandError(GenericResponse);

impl CommandError {
	#[tracing::instrument(name = "CommandError", skip(app, interaction))]
	pub async fn new(app: App, interaction: &Interaction, err: Report) -> Result<Action> {
		let handler = GraphicalReportHandler::new_themed(GraphicalTheme::unicode_nocolor());
		let mut error = String::from("Error:\n```");
		handler
			.render_report(&mut error, err.as_ref())
			.into_diagnostic()?;
		error.extend(repeat('`').take(3));

		let member = Member::try_from(interaction)?;
		let err = Error::create(app.clone(), member, &error).await?;

		Ok(
			ActionClass::CommandError(Self(GenericResponse::from_interaction(
				interaction,
				GenericResponseData {
					ephemeral: true,
					embeds: vec![EmbedBuilder::new()
						.color(0xFF_00_00)
						.description(error)
						.validate()
						.into_diagnostic()?
						.build()],
					components: if app.config.discord.maintainer_id.is_some() {
						action_row(vec![Component::Button(Button {
							custom_id: Some(format!("debug:publish-error:{}", err.id)),
							disabled: false,
							emoji: None,
							label: Some("Report to maintainer".to_string()),
							style: ButtonStyle::Secondary,
							url: None,
						})])
					} else {
						vec![]
					},
					..Default::default()
				},
			)))
			.into(),
		)
	}

	pub async fn handle(self, Args { app, .. }: Args) -> Result<()> {
		app.send_response(self.0).await.map(drop)
	}
}
