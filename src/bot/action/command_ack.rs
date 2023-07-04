use miette::{IntoDiagnostic, Result};
use twilight_model::{
	application::interaction::Interaction,
	http::interaction::{InteractionResponse, InteractionResponseType, InteractionResponseData},
	id::{marker::InteractionMarker, Id},
	channel::message::MessageFlags,
};

use super::{Action, ActionClass, Args};

#[derive(Debug, Clone)]
pub struct CommandAck {
	pub id: Id<InteractionMarker>,
	pub token: String,
	pub ephemeral: bool,
}
impl CommandAck {
	#[tracing::instrument(name = "CommandAck", skip(interaction))]
	pub fn new(interaction: &Interaction) -> Action {
		ActionClass::CommandAck(Self {
			id: interaction.id,
			token: interaction.token.clone(),
			ephemeral: false,
		})
		.into()
	}

	#[tracing::instrument(name = "CommandAck:ephemeral", skip(interaction))]
	pub fn ephemeral(interaction: &Interaction) -> Action {
		ActionClass::CommandAck(Self {
			id: interaction.id,
			token: interaction.token.clone(),
			ephemeral: true,
		})
		.into()
	}

	pub async fn handle(self, Args { app, .. }: Args) -> Result<()> {
		app.interaction_client()
			.create_response(
				self.id,
				&self.token,
				&InteractionResponse {
					kind: InteractionResponseType::DeferredChannelMessageWithSource,
					data: if self.ephemeral {
						Some(InteractionResponseData {
							flags: Some(MessageFlags::EPHEMERAL),
							..Default::default()
						})
					} else {
						None
					},
				},
			)
			.await
			.into_diagnostic()?;
		Ok(())
	}
}
