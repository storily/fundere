use miette::{IntoDiagnostic, Result};
use twilight_model::{
	application::interaction::Interaction,
	http::interaction::{InteractionResponse, InteractionResponseType},
	id::{marker::InteractionMarker, Id},
};

use super::{Action, ActionClass, Args};

#[derive(Debug, Clone)]
pub struct CommandAck {
	pub id: Id<InteractionMarker>,
	pub token: String,
}
impl CommandAck {
	#[tracing::instrument(name = "CommandAck", skip(interaction))]
	pub fn new(interaction: &Interaction) -> Action {
		ActionClass::CommandAck(Self {
			id: interaction.id,
			token: interaction.token.clone(),
		})
		.into()
	}

	pub async fn handle(self, Args { app, .. }: Args) -> Result<()> {
		app.interaction_client()
			.create_response(
				self.id,
				&self.token,
				&InteractionResponse {
					kind: InteractionResponseType::DeferredUpdateMessage,
					data: None,
				},
			)
			.exec()
			.await
			.into_diagnostic()?;
		Ok(())
	}
}
