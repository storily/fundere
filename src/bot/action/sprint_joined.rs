use miette::Result;
use twilight_model::{
	channel::message::component::{ButtonStyle, Button, Component},
	application::
	interaction::Interaction,
};

use crate::{
	bot::{
		context::{GenericResponse, GenericResponseData},
		utils::{action_row, time::ChronoDateTimeExt},
	},
	db::sprint::Sprint,
};

use super::{Action, ActionClass, Args};

#[derive(Debug, Clone)]
pub struct SprintJoined(GenericResponse);

impl SprintJoined {
	#[tracing::instrument(name = "SprintJoined", skip(interaction))]
	pub fn new(interaction: &Interaction, sprint: &Sprint) -> Result<Action> {
		let Sprint { id, shortid, .. } = sprint;
		Ok(ActionClass::SprintJoined(Self(GenericResponse::from_interaction(
			interaction,
			GenericResponseData {
				ephemeral: true,
				content: Some(format!("You've joined sprint `{shortid}`!")),
				components: action_row(vec![
					Component::Button(Button {
						custom_id: Some(format!("sprint:start-words:{id}")),
						disabled: false,
						emoji: None,
						label: Some("Record starting words".to_string()),
						style: ButtonStyle::Primary,
						url: None,
					}),
					Component::Button(Button {
						custom_id: Some(format!("sprint:leave:{id}")),
						disabled: false,
						emoji: None,
						label: Some("Leave".to_string()),
						style: ButtonStyle::Danger,
						url: None,
					}),
				]),
				..Default::default()
			},
		).with_age(sprint.created_at.elapsed()?)))
		.into())
	}

	pub async fn handle(self, Args { app, .. }: Args) -> Result<()> {
		app.send_response(self.0).await.map(drop)
	}
}
