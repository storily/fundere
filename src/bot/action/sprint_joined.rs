use miette::Result;
use twilight_model::application::{
	component::{button::ButtonStyle, Button, Component},
	interaction::Interaction,
};

use crate::{
	bot::{
		context::{GenericResponse, GenericResponseData},
		utils::action_row,
	},
	db::sprint::Sprint,
};

use super::{Action, ActionClass, Args};

#[derive(Debug, Clone)]
pub struct SprintJoined(GenericResponse);

impl SprintJoined {
	#[tracing::instrument(name = "SprintJoined", skip(interaction))]
	pub fn new(interaction: &Interaction, sprint: &Sprint) -> Action {
		let Sprint { id, shortid, .. } = sprint;
		ActionClass::SprintJoined(Self(GenericResponse::from_interaction(
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
		)))
		.into()
	}

	pub async fn handle(self, Args { app, .. }: Args) -> Result<()> {
		app.send_response(self.0).await.map(drop)
	}
}
