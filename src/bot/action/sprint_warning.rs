use humantime::format_duration;
use itertools::Itertools;
use miette::{miette, IntoDiagnostic, Result};
use twilight_http::client::InteractionClient;
use twilight_mention::Mention;
use twilight_model::{
	application::{
		component::{button::ButtonStyle, Button, Component},
		interaction::Interaction,
	},
	id::{marker::InteractionMarker, Id},
};
use uuid::Uuid;

use crate::{
	bot::{utils::action_row, App},
	db::sprint::{Sprint, SprintStatus},
};

use super::Action;

#[derive(Debug, Clone)]
pub struct SprintWarning {
	pub id: Id<InteractionMarker>,
	pub token: String,
	pub sprint: Uuid,
}

impl SprintWarning {
	#[tracing::instrument(name = "SprintWarning", skip(interaction))]
	pub fn new(interaction: &Interaction, sprint: Uuid) -> Action {
		Action::SprintWarning(Self {
			id: interaction.id,
			token: interaction.token.clone(),
			sprint,
		})
	}

	pub async fn handle(self, app: App, interaction_client: &InteractionClient<'_>) -> Result<()> {
		let sprint = Sprint::get_current(app.clone(), self.sprint).await?;
		if sprint.status >= SprintStatus::Started {
			return Err(miette!(
				"Bug: went to warn sprint but it was already started"
			));
		}

		let Sprint { id, shortid, .. } = sprint;
		let duration = format_duration(sprint.duration());
		let starting_in = format_duration(
			sprint
				.starting_in()
				.ok_or(miette!("Bug: sprint start is in the past"))?,
		);

		let participant_list = sprint
			.participants(app.clone())
			.await?
			.iter()
			.map(|p| p.mention().to_string())
			.join(", ");

		let content = format!(
			"⏱️ Sprint `{shortid}` is starting in {starting_in} for {duration}! // {participant_list}"
		);
		// TODO: ding

		interaction_client
			.create_followup(&self.token)
			.content(&content)
			.into_diagnostic()?
			.components(&action_row(vec![
				Component::Button(Button {
					custom_id: Some(format!("sprint:join:{id}")),
					disabled: false,
					emoji: None,
					label: Some("Join".to_string()),
					style: ButtonStyle::Secondary,
					url: None,
				}),
				Component::Button(Button {
					custom_id: Some(format!("sprint:start-words:{id}")),
					disabled: false,
					emoji: None,
					label: Some("Starting words".to_string()),
					style: ButtonStyle::Primary,
					url: None,
				}),
			]))
			.into_diagnostic()?
			.exec()
			.await
			.into_diagnostic()?;

		Ok(())
	}
}
