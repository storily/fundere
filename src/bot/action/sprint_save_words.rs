use miette::Result;
use twilight_model::{
	channel::message::component::{ButtonStyle, Button, Component},
	application::
	interaction::Interaction,
};

use crate::{
	bot::{
	App,
		context::{GenericResponse, GenericResponseData},
		utils::{action_row},
	},
	db::{project::Project, sprint::Sprint, member::Member, nanowrimo_login::NanowrimoLogin},
};

use super::{Action, ActionClass, Args};

#[derive(Debug, Clone)]
pub struct SprintSaveWords(GenericResponse);

impl SprintSaveWords {
	#[tracing::instrument(name = "SprintSaveWords", skip(interaction))]
	pub async fn new(app: App, interaction: &Interaction, sprint: &Sprint, member: Member) -> Result<Option<Action>> {
		let Some(project) = Project::get_for_member(app.clone(), member)
			.await? else {
	return Ok(None);
			};

		let Some(login) = NanowrimoLogin::get_for_member(app.clone(), member)
			.await?
			else {
				return Ok(None);
			};

		let participant = sprint.participant(app.clone(), member).await?;
		let Some(diff) = participant.words_written() else {
			return Ok(None);
		};

		if diff == 0 {
			return Ok(None);
		}

		Ok(Some(ActionClass::SprintSaveWords(Self(GenericResponse::from_interaction(
			interaction,
			GenericResponseData {
				ephemeral: true,
				content: Some(format!("Add {diff} words to nanowrimo.org?")),
				components: action_row(vec![
					Component::Button(Button {
						custom_id: Some(format!("sprint:save-words:{}:{}", sprint.id, project.id)),
						disabled: false,
						emoji: None,
						label: Some("Yes".to_string()),
						style: ButtonStyle::Primary,
						url: None,
					}),
					Component::Button(Button {
						custom_id: Some(format!("sprint:ask-not:{}", login.id)),
						disabled: false,
						emoji: None,
						label: Some("Don't ask me again".to_string()),
						style: ButtonStyle::Danger,
						url: None,
					}),
				]),
				..Default::default()
			},
		)))
		.into()))
	}

	pub async fn handle(self, Args { app, .. }: Args) -> Result<()> {
		app.send_response(self.0).await.map(drop)
	}
}
