use itertools::Itertools;
use miette::{miette, IntoDiagnostic, Result};
use twilight_mention::Mention;
use twilight_model::application::component::{button::ButtonStyle, Button, Component};
use uuid::Uuid;

use crate::{
	bot::utils::action_row,
	db::sprint::{Sprint, SprintStatus},
};

use super::{Action, ActionClass, Args};

#[derive(Debug, Clone)]
pub struct SprintEnd {
	pub token: String,
	pub sprint: Uuid,
}

impl SprintEnd {
	#[tracing::instrument(name = "SprintEnd")]
	pub fn new(sprint: &Sprint) -> Action {
		ActionClass::SprintEnd(Self {
			token: sprint.interaction_token.clone(),
			sprint: sprint.id,
		})
		.into()
	}

	pub async fn handle(
		self,
		Args {
			app,
			interaction_client,
			..
		}: Args<'_>,
	) -> Result<()> {
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
