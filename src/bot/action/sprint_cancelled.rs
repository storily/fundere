use miette::{IntoDiagnostic, Result};
use twilight_mention::Mention;
use twilight_model::{
	application::interaction::Interaction,
	http::interaction::{InteractionResponse, InteractionResponseType},
	id::{marker::InteractionMarker, Id},
	user::User,
};
use twilight_util::builder::InteractionResponseDataBuilder;

use super::{Action, ActionClass, Args};

#[derive(Debug, Clone)]
pub struct SprintCancelled {
	pub id: Id<InteractionMarker>,
	pub token: String,
	pub content: String,
}

impl SprintCancelled {
	#[tracing::instrument(name = "SprintCancelled", skip(interaction))]
	pub fn new(interaction: &Interaction, shortid: i32, user: &User) -> Action {
		ActionClass::SprintCancelled(Self {
			id: interaction.id,
			token: interaction.token.clone(),
			content: format!("❌ Sprint {shortid} was cancelled by {}", user.id.mention()),
		})
		.into()
	}

	pub async fn handle(
		self,
		Args {
			interaction_client,
			as_followup,
			..
		}: Args<'_>,
	) -> Result<()> {
		if as_followup {
			interaction_client
				.create_followup(&self.token)
				.content(&self.content)
				.into_diagnostic()?
				.exec()
				.await
				.into_diagnostic()
				.map(drop)
		} else {
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
				.into_diagnostic()
				.map(drop)
		}
	}
}
