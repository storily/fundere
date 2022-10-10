use itertools::Itertools;
use miette::{miette, IntoDiagnostic, Result};
use tracing::debug;
use twilight_mention::Mention;
use twilight_model::application::component::{button::ButtonStyle, Button, Component};
use uuid::Uuid;

use crate::{
	bot::{context::Timer, utils::action_row},
	db::sprint::{Sprint, SprintStatus},
};

use super::{Action, ActionClass, Args, SprintEnd};

#[derive(Debug, Clone)]
pub struct SprintStart {
	pub token: String,
	pub sprint: Uuid,
}

impl SprintStart {
	#[tracing::instrument(name = "SprintStart")]
	pub fn new(sprint: &Sprint) -> Action {
		ActionClass::SprintStart(Self {
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
			return Err(miette!("Bug: went to start sprint but it was already"));
		}

		let Sprint { id, shortid, .. } = sprint;
		let duration = sprint.formatted_duration();

		let participant_list = sprint
			.participants(app.clone())
			.await?
			.iter()
			.map(|p| p.mention().to_string())
			.join(", ");

		let ending_at = sprint
			.ending_at()
			.with_timezone(&chrono_tz::Pacific::Auckland)
			.format("%H:%M:%S");

		let content = format!("⏱️ Sprint `{shortid}` is starting now for {duration}! (ending at {ending_at}) // {participant_list}");
		// TODO: ding

		sprint
			.update_status(app.clone(), SprintStatus::Started)
			.await?;

		if let Ok(ending_in) = sprint.ending_in().to_std() {
			debug!("set up sprint end timer");
			app.send_timer(Timer::new_after(ending_in, SprintEnd::new(&sprint))?)
				.await?;
		} else {
			return Err(miette!("sprint ended before it began???"));
		}

		interaction_client
			.create_followup(&self.token)
			.content(&content)
			.into_diagnostic()?
			.components(&action_row(vec![
				Component::Button(Button {
					custom_id: Some(format!("sprint:join:{id}")),
					disabled: false,
					emoji: None,
					label: Some("Join late".to_string()),
					style: ButtonStyle::Secondary,
					url: None,
				}),
				Component::Button(Button {
					custom_id: Some(format!("sprint:start-words:{id}")),
					disabled: false,
					emoji: None,
					label: Some("Starting words".to_string()),
					style: ButtonStyle::Secondary,
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
