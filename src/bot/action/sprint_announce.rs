use std::{iter::once, str::FromStr, time::Duration};

use chrono::{DateTime, Utc};
use humantime::format_duration;
use miette::{miette, IntoDiagnostic, Result};
use sqlx::{postgres::types::PgInterval, types::Uuid, PgPool};
use strum::{Display, EnumString};
use twilight_http::client::InteractionClient;
use twilight_model::{
	application::{
		component::{button::ButtonStyle, ActionRow, Button, Component},
		interaction::Interaction,
	},
	http::interaction::{InteractionResponse, InteractionResponseType},
	id::{marker::InteractionMarker, Id},
};
use twilight_util::builder::InteractionResponseDataBuilder;

use crate::bot::App;

use super::Action;

#[derive(Debug, Clone)]
pub struct SprintAnnounce {
	pub id: Id<InteractionMarker>,
	pub token: String,
	pub sprint: Uuid,
	pub content: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, EnumString, Display)]
#[strum(serialize_all = "lowercase")]
pub enum SprintStatus {
	Initial,
	Announced,
	Started,
	Ended,
	Summaried,
}

#[derive(Debug, Clone, sqlx::FromRow)]
pub struct Sprint {
	pub id: Uuid,
	pub shortid: i32,
	pub starting_at: DateTime<Utc>,
	pub duration: PgInterval,
	pub status: String,
}

impl Sprint {
	pub async fn update_status(&self, pool: &PgPool, status: SprintStatus) -> Result<()> {
		sqlx::query("UPDATE sprints SET status = $2 WHERE id = $1")
			.bind(self.id)
			.bind(status.to_string())
			.execute(pool)
			.await
			.into_diagnostic()?;
		Ok(())
	}

	pub fn status(&self) -> Result<SprintStatus> {
		SprintStatus::from_str(&self.status).into_diagnostic()
	}

	pub fn duration(&self) -> Duration {
		Duration::from_secs(
			(self.duration.days as u64 + self.duration.months as u64 * 31)
				* 24 * 60 * 60 * 1_000_000
				+ (self.duration.microseconds as u64) / (1_000_000),
		)
	}

	pub fn starting_in(&self) -> Option<Duration> {
		let now = Utc::now();
		if self.starting_at > now {
			Some(Duration::from_secs(
				(self.starting_at - now).num_seconds() as _
			))
		} else {
			None
		}
	}
}

impl SprintAnnounce {
	pub async fn new(app: App, interaction: &Interaction, sprint: Uuid) -> Result<Action> {
		let sprint: Sprint = sqlx::query_as("SELECT * FROM sprints_current WHERE id = $1")
			.bind(sprint)
			.fetch_one(&app.db)
			.await
			.into_diagnostic()?;

		if sprint.status()? >= SprintStatus::Announced {
			return Err(miette!("Bug: went to announce sprint but it was already"));
		}

		let starting_at = sprint
			.starting_at
			.with_timezone(&chrono_tz::Pacific::Auckland)
			.format("%H:%M");
		let starting_in = format_duration(
			sprint
				.starting_in()
				.ok_or(miette!("Bug: sprint start is in the past"))?,
		);

		let duration = format_duration(sprint.duration());
		let content = format!(
			"⏱️ New sprint! Starting in {starting_in} (at {starting_at}), going for {duration}."
		);

		sprint
			.update_status(&app.db, SprintStatus::Announced)
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
									custom_id: Some(format!("sprint:announce:join:{sprint_id}")),
									disabled: false,
									emoji: None,
									label: Some("Join".to_string()),
									style: ButtonStyle::Primary,
									url: None,
								}),
								Component::Button(Button {
									custom_id: Some(format!("sprint:announce:cancel:{sprint_id}")),
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

fn action_row(components: Vec<Component>) -> impl Iterator<Item = Component> {
	once(Component::ActionRow(ActionRow { components }))
}
