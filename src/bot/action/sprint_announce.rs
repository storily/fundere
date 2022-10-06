use std::time::Duration;

use humantime::format_duration;
use miette::{miette, IntoDiagnostic, Result};
use tracing::debug;
use twilight_http::client::InteractionClient;
use twilight_model::{
	application::{
		component::{button::ButtonStyle, Button, Component},
		interaction::Interaction,
	},
	http::interaction::{InteractionResponse, InteractionResponseType},
	id::{marker::InteractionMarker, Id},
};
use twilight_util::builder::InteractionResponseDataBuilder;
use uuid::Uuid;

use crate::{
	bot::{
		action::{SprintStart, SprintWarning},
		context::Timer,
		utils::action_row,
		App,
	},
	db::sprint::{Sprint, SprintStatus},
};

use super::Action;

#[derive(Debug, Clone)]
pub struct SprintAnnounce {
	pub id: Id<InteractionMarker>,
	pub token: String,
	pub sprint: Uuid,
	pub content: String,
}

impl SprintAnnounce {
	#[tracing::instrument(name = "SprintAnnounce", skip(app, interaction))]
	pub async fn new(app: App, interaction: &Interaction, sprint: Sprint) -> Result<Action> {
		if sprint.status >= SprintStatus::Announced {
			return Err(miette!("Bug: went to announce sprint but it was already"));
		}

		let starting_at = sprint
			.starting_at
			.with_timezone(&chrono_tz::Pacific::Auckland)
			.format("%H:%M:%S");

		let starting_in = sprint
			.starting_in()
			.ok_or(miette!("Bug: sprint start is in the past"))?;

		let shortid = sprint.shortid;
		let duration = format_duration(sprint.duration());
		let starting_in_disp = format_duration(starting_in);
		let content = format!(
			"⏱️  New sprint! `{shortid}` is starting in {starting_in_disp} (at {starting_at}), going for {duration}."
		);

		sprint
			.update_status(app.clone(), SprintStatus::Announced)
			.await?;

		if starting_in > Duration::from_secs(60) {
			debug!("set up sprint warn timer");
			app.send_timer(Timer::new_after(
				starting_in - Duration::from_secs(30),
				SprintWarning::new(interaction, sprint.id),
			)?)
			.await?;
		}

		debug!("set up sprint start timer");
		app.send_timer(Timer::new_after(
			starting_in,
			SprintStart::new(interaction, sprint.id),
		)?)
		.await?;

		Ok(Action::SprintAnnounce(Self {
			id: interaction.id,
			token: interaction.token.clone(),
			sprint: sprint.id,
			content,
		}))
	}

	pub async fn handle(self, interaction_client: &InteractionClient<'_>) -> Result<()> {
		let sprint_id = self.sprint;
		interaction_client
			.create_response(
				self.id,
				&self.token,
				&InteractionResponse {
					kind: InteractionResponseType::ChannelMessageWithSource,
					data: Some(
						InteractionResponseDataBuilder::new()
							.content(self.content)
							.components(action_row(vec![
								Component::Button(Button {
									custom_id: Some(format!("sprint:join:{sprint_id}")),
									disabled: false,
									emoji: None,
									label: Some("Join".to_string()),
									style: ButtonStyle::Primary,
									url: None,
								}),
								Component::Button(Button {
									custom_id: Some(format!("sprint:cancel:{sprint_id}")),
									disabled: false,
									emoji: None,
									label: Some("Cancel".to_string()),
									style: ButtonStyle::Danger,
									url: None,
								}),
							]))
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
