use miette::{IntoDiagnostic, Result};
use twilight_model::{
	application::interaction::Interaction,
	channel::message::MessageFlags,
	http::interaction::{InteractionResponse, InteractionResponseType},
	id::{marker::InteractionMarker, Id},
};
use twilight_util::builder::InteractionResponseDataBuilder;
use uuid::Uuid;

use crate::db::sprint::Sprint;

use super::{Action, ActionClass, Args};

#[derive(Debug, Clone)]
pub struct SprintLeft {
	pub id: Id<InteractionMarker>,
	pub token: String,
	pub sprint_id: Uuid,
	pub shortid: i32,
}

impl SprintLeft {
	#[tracing::instrument(name = "SprintLeft", skip(interaction))]
	pub fn new(interaction: &Interaction, sprint: &Sprint) -> Action {
		ActionClass::SprintLeft(Self {
			id: interaction.id,
			token: interaction.token.clone(),
			sprint_id: sprint.id,
			shortid: sprint.shortid,
		})
		.into()
	}

	pub async fn handle(self, Args { app, .. }: Args) -> Result<()> {
		let Self { shortid, .. } = self;
		app.interaction_client()
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
