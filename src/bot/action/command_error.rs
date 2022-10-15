use std::iter::repeat;

use miette::{GraphicalReportHandler, GraphicalTheme, IntoDiagnostic, Report, Result};
use twilight_model::application::interaction::Interaction;
use twilight_util::builder::embed::EmbedBuilder;

use crate::bot::context::{GenericResponse, GenericResponseData};

use super::{Action, ActionClass, Args};

#[derive(Debug, Clone)]
pub struct CommandError(GenericResponse);

impl CommandError {
	#[tracing::instrument(name = "CommandError", skip(interaction))]
	pub fn new(interaction: &Interaction, err: Report) -> Result<Action> {
		let handler = GraphicalReportHandler::new_themed(GraphicalTheme::unicode_nocolor());
		let mut error = String::from("Error:\n```");
		handler
			.render_report(&mut error, err.as_ref())
			.into_diagnostic()?;
		error.extend(repeat('`').take(3));

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
					..Default::default()
				},
			)))
			.into(),
		)
	}

	pub async fn handle(self, Args { app, .. }: Args) -> Result<()> {
		app.send_response(self.0).await
	}
}
