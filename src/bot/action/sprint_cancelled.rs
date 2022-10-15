use miette::Result;
use twilight_mention::Mention;
use twilight_model::{application::interaction::Interaction, user::User};

use crate::bot::context::{GenericResponse, GenericResponseData};

use super::{Action, ActionClass, Args};

#[derive(Debug, Clone)]
pub struct SprintCancelled(GenericResponse);

impl SprintCancelled {
	#[tracing::instrument(name = "SprintCancelled", skip(interaction))]
	pub fn new(interaction: &Interaction, shortid: i32, user: &User) -> Action {
		ActionClass::SprintCancelled(Self(GenericResponse::from_interaction(
			interaction,
			GenericResponseData {
				content: Some(format!(
					"❌ Sprint {shortid} was cancelled by {}",
					user.id.mention()
				)),
				..Default::default()
			},
		)))
		.into()
	}

	pub async fn handle(self, Args { app, .. }: Args) -> Result<()> {
		app.send_response(self.0).await.map(drop)
	}
}
