use miette::Result;
use tracing::debug;
use twilight_model::application::interaction::Interaction;

use crate::{
	bot::{
		context::{GenericResponse, GenericResponseData},
		utils::time::ChronoDateTimeExt,
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
		update_status(&sprint, app.clone()).await?;

		let summary = sprint.summary_text(app).await?;

		Ok(ActionClass::SprintSummary(Box::new(Self(
			GenericResponse::from_interaction(
				interaction,
				GenericResponseData {
					content: Some(summary),
					..Default::default()
				},
			)
			.with_age(sprint.created_at.elapsed()?),
		)))
		.into())
	}

	#[tracing::instrument(name = "SprintSummary::new_from_db", skip(app))]
	pub async fn new_from_db(app: App, sprint: Sprint) -> Result<Action> {
		update_status(&sprint, app.clone()).await?;

		let summary = sprint.summary_text(app).await?;
		debug!("got summary, let's post it");

		Ok(
			ActionClass::SprintSummary(Box::new(Self(GenericResponse::from_sprint(
				&sprint,
				GenericResponseData {
					content: Some(summary),
					..Default::default()
				},
			))))
			.into(),
		)
	}

	#[tracing::instrument(name = "SprintSummary::handle", skip(app))]
	pub async fn handle(self, Args { app, .. }: Args) -> Result<()> {
		app.send_response(self.0).await.map(drop)
	}
}

async fn update_status(sprint: &Sprint, app: App) -> Result<()> {
	if sprint.status == SprintStatus::Ended {
		debug!("sprint ended, marking it as summaried");
		sprint.update_status(app, SprintStatus::Summaried).await?;
	} else if sprint.status < SprintStatus::Ended {
		debug!("sprint not ended, summary will probably be partial");
	} else {
		debug!("sprint already summaried, showing again");
	}

	Ok(())
}
