use miette::Result;
use twilight_mention::Mention;
use twilight_model::{application::interaction::Interaction, user::User};

use crate::{
	bot::{
		context::{GenericResponse, GenericResponseData},
		utils::time::ChronoDateTimeExt,
	},
	db::sprint::Sprint,
};

use super::{Action, ActionClass, Args};

#[derive(Debug, Clone)]
pub struct SprintCancelled(GenericResponse);

impl SprintCancelled {
	#[tracing::instrument(name = "SprintCancelled", skip(interaction))]
	pub fn new(interaction: &Interaction, sprint: &Sprint, user: &User) -> Result<Action> {
		Ok(ActionClass::SprintCancelled(Box::new(Self(
			GenericResponse::from_interaction(
				interaction,
				GenericResponseData {
					content: Some(format!(
						"❌ Sprint {} was cancelled by {}",
						sprint.shortid,
						user.id.mention()
					)),
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
