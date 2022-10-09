use miette::{IntoDiagnostic, Result};
use twilight_http::client::InteractionClient;
use twilight_model::{
	application::interaction::Interaction,
	http::interaction::{InteractionResponse, InteractionResponseType},
	id::{marker::InteractionMarker, Id},
};

use super::Action;

#[derive(Debug, Clone)]
pub struct CommandAck {
	pub id: Id<InteractionMarker>,
	pub token: String,
}
impl CommandAck {
	#[tracing::instrument(name = "CommandAck", skip(interaction))]
	pub fn new(interaction: &Interaction) -> Action {
		Action::CommandAck(Self {
			id: interaction.id,
			token: interaction.token.clone(),
		})
	}

	pub async fn handle(self, interaction_client: &InteractionClient<'_>) -> Result<()> {
		interaction_client
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
