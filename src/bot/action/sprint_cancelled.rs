use miette::{IntoDiagnostic, Result};
use twilight_http::client::InteractionClient;
use twilight_mention::Mention;
use twilight_model::{
	application::interaction::Interaction,
	http::interaction::{InteractionResponse, InteractionResponseType},
	id::{marker::InteractionMarker, Id},
	user::User,
};
use twilight_util::builder::InteractionResponseDataBuilder;

use super::Action;

#[derive(Debug, Clone)]
pub struct SprintCancelled {
	pub id: Id<InteractionMarker>,
	pub token: String,
	pub content: String,
}

impl SprintCancelled {
	#[tracing::instrument(skip(interaction))]
	pub fn new(interaction: &Interaction, user: &User) -> Action {
		Action::SprintCancelled(Self {
			id: interaction.id,
			token: interaction.token.clone(),
			content: format!("❌ Sprint was cancelled by {}", user.id.mention()),
		})
	}

	pub async fn handle(self, interaction_client: &InteractionClient<'_>) -> Result<()> {
		interaction_client
			.create_response(
				self.id,
				&self.token,
				&InteractionResponse {
					kind: InteractionResponseType::ChannelMessageWithSource,
					data: Some(
						InteractionResponseDataBuilder::new()
							.content(self.content)
							.build(),
					),
				},
			)
			.exec()
			.await
			.into_diagnostic()?;
		Ok(())
	}
}
