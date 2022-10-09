use itertools::Itertools;
use miette::{miette, IntoDiagnostic, Result};
use twilight_http::client::InteractionClient;
use twilight_mention::Mention;
use twilight_model::{
	application::component::{button::ButtonStyle, Button, Component},
	id::{marker::InteractionMarker, Id},
};
use uuid::Uuid;

use crate::{
	bot::{utils::action_row, App},
	db::sprint::{Sprint, SprintStatus},
};

use super::Action;

#[derive(Debug, Clone)]
pub struct SprintEnd {
	pub id: Id<InteractionMarker>,
	pub token: String,
	pub sprint: Uuid,
}

impl SprintEnd {
	#[tracing::instrument(name = "SprintEnd", skip(token))]
	pub fn new(id: Id<InteractionMarker>, token: &str, sprint: Uuid) -> Action {
		Action::SprintEnd(Self {
			id,
			token: token.to_string(),
			sprint,
		})
	}

	pub async fn handle(self, app: App, interaction_client: &InteractionClient<'_>) -> Result<()> {
		let sprint = Sprint::get(app.clone(), self.sprint).await?;
		if sprint.status >= SprintStatus::Ended {
			return Err(miette!("Bug: went to end sprint but it was already"));
		}

		let Sprint { id, shortid, .. } = sprint;

		let participant_list = sprint
			.participants(app.clone())
			.await?
			.iter()
			.map(|p| p.mention().to_string())
			.join(", ");

		let content = format!("⏱️ Stop writing! Sprint `{shortid}` is done. // {participant_list}");

		sprint
			.update_status(app.clone(), SprintStatus::Ended)
			.await?;

		interaction_client
			.create_followup(&self.token)
			.content(&content)
			.into_diagnostic()?
			.components(&action_row(vec![Component::Button(Button {
				custom_id: Some(format!("sprint:end-words:{id}")),
				disabled: false,
				emoji: None,
				label: Some("Ending words".to_string()),
				style: ButtonStyle::Secondary,
				url: None,
			})]))
			.into_diagnostic()?
			.exec()
			.await
			.into_diagnostic()?;

		Ok(())
	}
}
