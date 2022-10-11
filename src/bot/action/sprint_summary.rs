use miette::{miette, IntoDiagnostic, Result};
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
	pub id: Option<Id<InteractionMarker>>,
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
			id: Some(interaction.id),
			token: interaction.token.clone(),
			sprint: sprint.id,
			content: sprint.summary_text(app).await?,
		})
		.into())
	}

	#[tracing::instrument(name = "SprintSummary::new_from_db", skip(app))]
	pub async fn new_from_db(app: App, sprint: Sprint) -> Result<Action> {
		sprint
			.update_status(app.clone(), SprintStatus::Summaried)
			.await?;

		Ok(Action::from(ActionClass::SprintSummary(Self {
			id: None,
			token: sprint.interaction_token.clone(),
			sprint: sprint.id,
			content: sprint.summary_text(app).await?,
		}))
		.as_followup())
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
		} else if let Some(interaction_id) = self.id {
			interaction_client
				.create_response(
					interaction_id,
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
		} else {
			Err(miette!(
				"cannot handle interaction with no id or not a followup"
			))
		}
	}
}
