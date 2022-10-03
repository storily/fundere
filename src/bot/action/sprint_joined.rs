use miette::{IntoDiagnostic, Result};
use sqlx::types::Uuid;
use twilight_http::client::InteractionClient;
use twilight_model::{
	application::{
		component::{button::ButtonStyle, Button, Component},
		interaction::Interaction,
	},
	http::interaction::{InteractionResponse, InteractionResponseType},
	id::{marker::InteractionMarker, Id}, channel::message::MessageFlags,
};
use twilight_util::builder::InteractionResponseDataBuilder;

use crate::{bot::utils::action_row, db::sprint::Sprint};

use super::Action;

#[derive(Debug, Clone)]
pub struct SprintJoined {
	pub id: Id<InteractionMarker>,
	pub token: String,
	pub sprint: Uuid,
}

impl SprintJoined {
	pub fn new(interaction: &Interaction, sprint: Sprint) -> Action {
		Action::SprintJoined(Self {
			id: interaction.id,
			token: interaction.token.clone(),
			sprint: sprint.id,
		})
	}

	pub async fn handle(self, interaction_client: &InteractionClient<'_>) -> Result<()> {
		let sprint_id = self.sprint;
		interaction_client
			.create_response(
				self.id,
				&self.token,
				&InteractionResponse {
					kind: InteractionResponseType::ChannelMessageWithSource,
					data: Some(
						InteractionResponseDataBuilder::new()
							.content("You've joined the sprint!")
							.flags(MessageFlags::EPHEMERAL)
							.components(action_row(vec![
								Component::Button(Button {
									custom_id: Some(format!(
										"sprint:joined:start-words:{sprint_id}"
									)),
									disabled: false,
									emoji: None,
									label: Some("Starting words".to_string()),
									style: ButtonStyle::Primary,
									url: None,
								}),
								Component::Button(Button {
									custom_id: Some(format!("sprint:joined:leave:{sprint_id}")),
									disabled: false,
									emoji: None,
									label: Some("Leave".to_string()),
									style: ButtonStyle::Danger,
									url: None,
								}),
							]))
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