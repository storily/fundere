use humantime::format_duration;
use miette::{miette, IntoDiagnostic, Result};
use sqlx::types::Uuid;
use twilight_http::client::InteractionClient;
use twilight_model::{
	application::{
		component::{button::ButtonStyle, Button, Component},
		interaction::Interaction,
	},
	id::{marker::InteractionMarker, Id},
};

use crate::{
	bot::{utils::action_row, App},
	db::sprint::{Sprint, SprintStatus},
};

use super::Action;

#[derive(Debug, Clone)]
pub struct SprintStart {
	pub id: Id<InteractionMarker>,
	pub token: String,
	pub sprint: Uuid,
}

impl SprintStart {
	#[tracing::instrument(name = "SprintStart", skip(interaction))]
	pub fn new(interaction: &Interaction, sprint: Uuid) -> Action {
		Action::SprintStart(Self {
			id: interaction.id,
			token: interaction.token.clone(),
			sprint,
		})
	}

	pub async fn handle(self, app: App, interaction_client: &InteractionClient<'_>) -> Result<()> {
		let sprint = Sprint::from_current(app.clone(), self.sprint).await?;
		if sprint.status()? >= SprintStatus::Started {
			return Err(miette!("Bug: went to start sprint but it was already"));
		}

		let Sprint { id, shortid, .. } = sprint;
		let duration = format_duration(sprint.duration());
		let content = format!("⏱️ Sprint `{shortid}` is starting now for {duration}!");
		// TODO: mention participants
		// TODO: ding
		// TODO: schedule end

		sprint
			.update_status(app.clone(), SprintStatus::Started)
			.await?;

		interaction_client
			.create_followup(&self.token)
			.content(&content).into_diagnostic()?
			.components(&action_row(vec![Component::Button(Button {
				custom_id: Some(format!("sprint:start:join:{id}")),
				disabled: false,
				emoji: None,
				label: Some("Join late".to_string()),
				style: ButtonStyle::Primary,
				url: None,
			})])).into_diagnostic()?
			.exec()
			.await
			.into_diagnostic()?;

		Ok(())
	}
}
