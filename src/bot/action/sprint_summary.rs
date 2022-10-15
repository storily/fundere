use miette::Result;
use twilight_model::application::interaction::Interaction;

use crate::{
	bot::{
		context::{GenericResponse, GenericResponseData},
		App,
	},
	db::sprint::{Sprint, SprintStatus},
};

use super::{Action, ActionClass, Args};

#[derive(Debug, Clone)]
pub struct SprintSummary(GenericResponse);

impl SprintSummary {
	#[tracing::instrument(name = "SprintSummary", skip(app, interaction))]
	pub async fn new(app: App, interaction: &Interaction, sprint: Sprint) -> Result<Action> {
		sprint
			.update_status(app.clone(), SprintStatus::Summaried)
			.await?;

		Ok(
			ActionClass::SprintSummary(Self(GenericResponse::from_interaction(
				interaction,
				GenericResponseData {
					content: Some(sprint.summary_text(app).await?),
					..Default::default()
				},
			)))
			.into(),
		)
	}

	#[tracing::instrument(name = "SprintSummary::new_from_db", skip(app))]
	pub async fn new_from_db(app: App, sprint: Sprint) -> Result<Action> {
		sprint
			.update_status(app.clone(), SprintStatus::Summaried)
			.await?;

		Ok(
			ActionClass::SprintSummary(Self(GenericResponse::from_sprint(
				&sprint,
				GenericResponseData {
					content: Some(sprint.summary_text(app).await?),
					..Default::default()
				},
			)))
			.into(),
		)
	}

	pub async fn handle(self, Args { app, .. }: Args) -> Result<()> {
		app.send_response(self.0).await.map(drop)
	}
}
