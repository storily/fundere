use chrono::Duration;
use humantime::format_duration;
use itertools::Itertools;
use miette::{miette, IntoDiagnostic, Result};
use twilight_mention::Mention;
use twilight_model::application::component::{button::ButtonStyle, Button, Component};
use uuid::Uuid;

use crate::{
	bot::utils::{action_row, time::round_duration_to_seconds},
	db::sprint::{Sprint, SprintStatus},
};

use super::{Action, ActionClass, Args};

#[derive(Debug, Clone)]
pub struct SprintWarning {
	pub token: String,
	pub sprint: Uuid,
}

impl SprintWarning {
	#[tracing::instrument(name = "SprintWarning")]
	pub fn new(sprint: &Sprint) -> Action {
		ActionClass::SprintWarning(Self {
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
		let sprint = Sprint::get_current(app.clone(), self.sprint).await?;
		if sprint.status >= SprintStatus::Started {
			return Err(miette!(
				"Bug: went to warn sprint but it was already started"
			));
		}

		let Sprint { id, shortid, .. } = sprint;
		let duration = sprint.formatted_duration();
		let starting_in = sprint.starting_in();
		let starting_in = if starting_in <= Duration::zero() {
			"now".into()
		} else {
			format!(
				"in {}",
				format_duration(round_duration_to_seconds(starting_in))
			)
		};

		let participant_list = sprint
			.participants(app.clone())
			.await?
			.iter()
			.map(|p| p.mention().to_string())
			.join(", ");

		let content = format!(
			"⏱️ Sprint `{shortid}` is starting {starting_in} for {duration}! // {participant_list}"
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
