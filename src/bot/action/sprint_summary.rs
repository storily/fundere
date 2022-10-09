use miette::{IntoDiagnostic, Result};
use twilight_model::{
	application::interaction::Interaction,
	http::interaction::{InteractionResponse, InteractionResponseType},
	id::{marker::InteractionMarker, Id},
};
use twilight_util::builder::InteractionResponseDataBuilder;
use uuid::Uuid;

use crate::{
	bot::App,
	db::sprint::{Sprint, SprintStatus},
};

use super::{Action, ActionClass, Args};

#[derive(Debug, Clone)]
pub struct SprintSummary {
	pub id: Id<InteractionMarker>,
	pub token: String,
	pub sprint: Uuid,
	pub content: String,
}

impl SprintSummary {
	#[tracing::instrument(name = "SprintSummary", skip(app, interaction))]
	pub async fn new(app: App, interaction: &Interaction, sprint: Sprint) -> Result<Action> {
		sprint
			.update_status(app.clone(), SprintStatus::Summaried)
			.await?;

		Ok(ActionClass::SprintSummary(Self {
			id: interaction.id,
			token: interaction.token.clone(),
			sprint: sprint.id,
			content: sprint.summary_text(app).await?,
		})
		.into())
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
