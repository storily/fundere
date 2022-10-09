use miette::{IntoDiagnostic, Result};
use twilight_model::{
	application::{
		component::{button::ButtonStyle, Button, Component},
		interaction::Interaction,
	},
	channel::message::MessageFlags,
	http::interaction::{InteractionResponse, InteractionResponseType},
	id::{marker::InteractionMarker, Id},
};
use twilight_util::builder::InteractionResponseDataBuilder;
use uuid::Uuid;

use crate::{bot::utils::action_row, db::sprint::Sprint};

use super::{Action, ActionClass, Args};

#[derive(Debug, Clone)]
pub struct SprintJoined {
	pub id: Id<InteractionMarker>,
	pub token: String,
	pub sprint_id: Uuid,
	pub shortid: i32,
}

impl SprintJoined {
	#[tracing::instrument(name = "SprintJoined", skip(interaction))]
	pub fn new(interaction: &Interaction, sprint: Sprint) -> Action {
		ActionClass::SprintJoined(Self {
			id: interaction.id,
			token: interaction.token.clone(),
			sprint_id: sprint.id,
			shortid: sprint.shortid,
		})
		.into()
	}

	pub async fn handle(
		self,
		Args {
			interaction_client, ..
		}: Args<'_>,
	) -> Result<()> {
		let Self {
			sprint_id, shortid, ..
		} = self;
		interaction_client
			.create_response(
				self.id,
				&self.token,
				&InteractionResponse {
					kind: InteractionResponseType::ChannelMessageWithSource,
					data: Some(
						InteractionResponseDataBuilder::new()
							.content(format!("You've joined sprint `{shortid}`!"))
							.flags(MessageFlags::EPHEMERAL)
							.components(action_row(vec![
								Component::Button(Button {
									custom_id: Some(format!("sprint:start-words:{sprint_id}")),
									disabled: false,
									emoji: None,
									label: Some("Record starting words early".to_string()),
									style: ButtonStyle::Primary,
									url: None,
								}),
								Component::Button(Button {
									custom_id: Some(format!("sprint:leave:{sprint_id}")),
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
