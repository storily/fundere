use miette::{IntoDiagnostic, Result};
use sqlx::types::Uuid;
use twilight_http::client::InteractionClient;
use twilight_model::{
	application::interaction::Interaction,
	channel::message::MessageFlags,
	http::interaction::{InteractionResponse, InteractionResponseType},
	id::{marker::InteractionMarker, Id},
};
use twilight_util::builder::InteractionResponseDataBuilder;

use crate::db::sprint::Sprint;

use super::Action;

#[derive(Debug, Clone)]
pub struct SprintLeft {
	pub id: Id<InteractionMarker>,
	pub token: String,
	pub sprint_id: Uuid,
	pub shortid: i32,
}

impl SprintLeft {
	#[tracing::instrument(name = "SprintLeft", skip(interaction))]
	pub fn new(interaction: &Interaction, sprint: Sprint) -> Action {
		Action::SprintLeft(Self {
			id: interaction.id,
			token: interaction.token.clone(),
			sprint_id: sprint.id,
			shortid: sprint.shortid,
		})
	}

	pub async fn handle(self, interaction_client: &InteractionClient<'_>) -> Result<()> {
		let Self { shortid, .. } = self;
		interaction_client
			.create_response(
				self.id,
				&self.token,
				&InteractionResponse {
					kind: InteractionResponseType::ChannelMessageWithSource,
					data: Some(
						InteractionResponseDataBuilder::new()
							.content(format!("You've left sprint `{shortid}`."))
							.flags(MessageFlags::EPHEMERAL)
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