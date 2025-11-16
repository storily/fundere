use miette::Result;
use twilight_model::application::interaction::Interaction;

use crate::{
	bot::{
		context::{GenericResponse, GenericResponseData},
		utils::time::ChronoDateTimeExt,
	},
	db::sprint::Sprint,
};

use super::{Action, ActionClass, Args};

#[derive(Debug, Clone)]
pub struct SprintLeft(GenericResponse);

impl SprintLeft {
	#[tracing::instrument(name = "SprintLeft", skip(interaction))]
	pub fn new(interaction: &Interaction, sprint: &Sprint) -> Result<Action> {
		let Sprint { shortid, .. } = sprint;
		Ok(ActionClass::SprintLeft(Box::new(Self(
			GenericResponse::from_interaction(
				interaction,
				GenericResponseData {
					ephemeral: true,
					content: Some(format!("You've left sprint `{shortid}`.")),
					..Default::default()
				},
			)
			.with_age(sprint.created_at.elapsed()?),
		)))
		.into())
	}

	pub async fn handle(self, Args { app, .. }: Args) -> Result<()> {
		app.send_response(self.0).await.map(drop)
	}
}
