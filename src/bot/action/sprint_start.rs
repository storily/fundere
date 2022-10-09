use humantime::format_duration;
use itertools::Itertools;
use miette::{miette, IntoDiagnostic, Result};
use tracing::debug;
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
	bot::{action::SprintEnd, context::Timer, utils::action_row, App},
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
		let sprint = Sprint::get_current(app.clone(), self.sprint).await?;
		if sprint.status >= SprintStatus::Started {
			return Err(miette!("Bug: went to start sprint but it was already"));
		}

		let Sprint { id, shortid, .. } = sprint;
		let duration = format_duration(sprint.duration());

		let participant_list = sprint
			.participants(app.clone())
			.await?
			.iter()
			.map(|p| p.mention().to_string())
			.join(", ");

		let ending_at = sprint
			.ending_at()?
			.with_timezone(&chrono_tz::Pacific::Auckland)
			.format("%H:%M:%S");

		let content = format!("⏱️ Sprint `{shortid}` is starting now for {duration}! (ending at {ending_at}) // {participant_list}");
		// TODO: ding

		sprint
			.update_status(app.clone(), SprintStatus::Started)
			.await?;

		if let Some(ending_in) = sprint.ending_in() {
			debug!("set up sprint end timer");
			app.send_timer(Timer::new_after(
				ending_in,
				SprintEnd::new(self.id, &self.token, sprint.id),
			)?)
			.await?;
		} else {
			return Err(miette!("sprint ended before it began???"));
		}

		interaction_client
			.create_followup(&self.token)
			.content(&content)
			.into_diagnostic()?
			.components(&action_row(vec![Component::Button(Button {
				custom_id: Some(format!("sprint:join:{id}")),
				disabled: false,
				emoji: None,
				label: Some("Join late".to_string()),
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
