use std::{iter::repeat, time::Duration};

use miette::{Context, GraphicalReportHandler, GraphicalTheme, IntoDiagnostic, Report, Result};
use tokio::time::timeout;
use tracing::debug;
use twilight_http::error::ErrorType;
use twilight_model::{
	application::interaction::Interaction,
	channel::message::MessageFlags,
	http::interaction::{InteractionResponse, InteractionResponseType},
	id::{marker::InteractionMarker, Id},
};
use twilight_util::builder::{embed::EmbedBuilder, InteractionResponseDataBuilder};

use super::{Action, ActionClass, Args};

#[derive(Debug, Clone)]
pub struct CommandError {
	pub id: Id<InteractionMarker>,
	pub token: String,
	pub error: String,
}
impl CommandError {
	#[tracing::instrument(name = "CommandError", skip(interaction))]
	pub fn new(interaction: &Interaction, err: Report) -> Result<Action> {
		let handler = GraphicalReportHandler::new_themed(GraphicalTheme::unicode_nocolor());
		let mut error = String::from("Error:\n```");
		handler
			.render_report(&mut error, err.as_ref())
			.into_diagnostic()?;
		error.extend(repeat('`').take(3));
		Ok(ActionClass::CommandError(Self {
			id: interaction.id,
			token: interaction.token.clone(),
			error,
		})
		.into())
	}

	pub async fn handle(self, Args { app, .. }: Args) -> Result<()> {
		let ic = app.interaction_client();
		debug!("check if response already sent");
		let has_response = timeout(
			Duration::from_millis(app.config.internal.response_lookup_timeout),
			ic.response(&self.token).exec(),
		)
		.await
		.map_or_else(|_| Ok(false), |resp| resp.map(|_| true))
		.or_else(|err| match err.kind() {
			ErrorType::Response { status, .. } if status.get() == 404 => Ok(false),
			_ => Err(err),
		})
		.into_diagnostic()
		.wrap_err("has_response")?;

		let embeds = vec![EmbedBuilder::new()
			.color(0xFF_00_00)
			.description(self.error)
			.validate()
			.into_diagnostic()?
			.build()];

		if has_response {
			debug!("send followup");
			ic.create_followup(&self.token)
				.flags(MessageFlags::EPHEMERAL)
				.embeds(&embeds)
				.into_diagnostic()
				.wrap_err("followup embed")?
				.exec()
				.await
				.into_diagnostic()
				.wrap_err("followup exec")?
				.model()
				.await
				.into_diagnostic()
				.wrap_err("followup response")
				.map(drop)
		} else {
			debug!("send response");
			ic.create_response(
				self.id,
				&self.token,
				&InteractionResponse {
					kind: InteractionResponseType::ChannelMessageWithSource,
					data: Some(
						InteractionResponseDataBuilder::new()
							.flags(MessageFlags::EPHEMERAL)
							.embeds(embeds.into_iter())
							.build(),
					),
				},
			)
			.exec()
			.await
			.into_diagnostic()
			.wrap_err("create response")
			.map(drop)
		}
	}
}
