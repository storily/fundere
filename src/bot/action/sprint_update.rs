use miette::Result;
use twilight_mention::Mention;
use twilight_model::{application::interaction::Interaction, user::User};

use crate::bot::context::{GenericResponse, GenericResponseData};

use super::{Action, ActionClass, Args};

#[derive(Debug, Clone)]
pub struct SprintUpdate {
	sprint: Uuid,
}

impl SprintUpdate {
	#[tracing::instrument(name = "SprintUpdate", skip(interaction))]
	pub fn new(sprint: &Sprint) -> Action {
		ActionClass::SprintUpdate(Self {
			sprint: sprint.id,
		})
		.into()
	}

	pub async fn handle(self, Args { app, .. }: Args) -> Result<()> {
		warn!("todo");
		Ok(())
		// app.send_response(self.0).await.map(drop)
	}
}
