use itertools::Itertools;
use miette::{miette, Result};
use tracing::debug;
use twilight_mention::Mention;
use twilight_model::application::component::{button::ButtonStyle, Button, Component};
use uuid::Uuid;

use crate::{
	bot::{
		context::{GenericResponse, GenericResponseData},
		utils::action_row,
	},
	db::sprint::{Sprint, SprintStatus},
};

use super::{Action, ActionClass, Args};

#[derive(Debug, Clone)]
pub struct SprintEnd(Uuid);

impl SprintEnd {
	#[tracing::instrument(name = "SprintEnd")]
	pub fn new(sprint: &Sprint) -> Action {
		ActionClass::SprintEnd(Self(sprint.id)).into()
	}

	pub async fn handle(self, Args { app, .. }: Args) -> Result<()> {
		let sprint = Sprint::get(app.clone(), self.0).await?;
		if sprint.is_cancelled() {
			debug!("sprint was cancelled, not ending");
			return Ok(());
		}
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

		let components = action_row(vec![Component::Button(Button {
			custom_id: Some(format!("sprint:end-words:{id}")),
			disabled: false,
			emoji: None,
			label: Some("Ending words".to_string()),
			style: ButtonStyle::Secondary,
			url: None,
		})]);

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
