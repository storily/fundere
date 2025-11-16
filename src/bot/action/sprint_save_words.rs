use miette::Result;
use twilight_model::{
	application::interaction::Interaction,
	channel::message::component::{Button, ButtonStyle, Component},
};

use crate::{
	bot::{
		context::{GenericResponse, GenericResponseData},
		utils::action_row,
		App,
	},
	db::{member::Member, project::Project, sprint::Sprint, trackbear_login::TrackbearLogin},
};

use super::{Action, ActionClass, Args};

#[derive(Debug, Clone)]
pub struct SprintSaveWords(GenericResponse);

impl SprintSaveWords {
	#[tracing::instrument(name = "SprintSaveWords", skip(interaction))]
	pub async fn new(
		app: App,
		interaction: &Interaction,
		sprint: &Sprint,
		member: Member,
	) -> Result<Option<Action>> {
		let Some(project) = Project::get_for_member(app.clone(), member).await? else {
			return Ok(None);
		};

		let Some(login) = TrackbearLogin::get_for_member(app.clone(), member).await? else {
			return Ok(None);
		};

		if !login.ask_me {
			return Ok(None);
		}

		// Fetch project title from TrackBear API
		let trackbear_project = project.fetch(app.clone()).await?;
		let title = trackbear_project.title();

		let participant = sprint.participant(app.clone(), member).await?;
		let Some(diff) = participant.words_written() else {
			return Ok(None);
		};

		if diff == 0 {
			return Ok(None);
		}

		Ok(Some(
			ActionClass::SprintSaveWords(Box::new(Self(GenericResponse::from_interaction(
				interaction,
				GenericResponseData {
					ephemeral: true,
					content: Some(format!("Save {diff:+} words to «{title}» on TrackBear?")),
					components: action_row(vec![
						Component::Button(Button {
							custom_id: Some(format!(
								"sprint:save-words:{}:{}",
								sprint.id, project.id
							)),
							disabled: false,
							emoji: None,
							label: Some("Yes please!".to_string()),
							style: ButtonStyle::Success,
							url: None,
						}),
						Component::Button(Button {
							custom_id: Some(format!("sprint:save-never:{}", login.id)),
							disabled: false,
							emoji: None,
							label: Some("Don't ask me again".to_string()),
							style: ButtonStyle::Danger,
							url: None,
						}),
					]),
					..Default::default()
				},
			))))
			.into(),
		))
	}

	pub async fn handle(self, Args { app, .. }: Args) -> Result<()> {
		app.send_response(self.0).await.map(drop)
	}
}
