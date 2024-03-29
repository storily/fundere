use chrono::Duration;
use itertools::Itertools;
use miette::{miette, Result};
use twilight_mention::Mention;
use twilight_model::channel::message::component::{ButtonStyle, Button, Component};
use uuid::Uuid;

use crate::{
	bot::{
		context::{GenericResponse, GenericResponseData},
		utils::{action_row, time::ChronoDateTimeExt},
	},
	db::sprint::{Sprint, SprintStatus},
};

use super::{Action, ActionClass, Args};

#[derive(Debug, Clone)]
pub struct SprintStartWarning(Uuid);

impl SprintStartWarning {
	#[tracing::instrument(name = "SprintStartWarning")]
	pub fn new(sprint: &Sprint) -> Action {
		ActionClass::SprintStartWarning(Self(sprint.id)).into()
	}

	pub async fn handle(self, Args { app, .. }: Args) -> Result<()> {
		let sprint = Sprint::get_current(app.clone(), self.0).await?;
		if sprint.status >= SprintStatus::Started {
			return Err(miette!(
				"Bug: went to warn sprint but it was already started"
			));
		}

		let Sprint { id, shortid, .. } = sprint;
		let duration = sprint.formatted_duration();
		let starting_in = if sprint.starting_in() <= Duration::zero() {
			"now".into()
		} else {
			sprint.starting_at.discord_format('R')
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

		let components = action_row(vec![
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
		]);

		app.send_response(GenericResponse::from_sprint(
			&sprint,
			GenericResponseData {
				content: Some(content),
				components,
				..Default::default()
			},
		))
		.await
		.map(drop)
	}
}
