use miette::Result;
use tracing::warn;
use uuid::Uuid;

use crate::db::sprint::Sprint;

use super::{Action, ActionClass, Args};

#[derive(Debug, Clone)]
pub struct SprintUpdate {
	sprint: Uuid,
}

impl SprintUpdate {
	#[tracing::instrument(name = "SprintUpdate")]
	pub fn new(sprint: &Sprint) -> Action {
		ActionClass::SprintUpdate(Self { sprint: sprint.id }).into()
	}

	pub async fn handle(self, Args { app, .. }: Args) -> Result<()> {
		let sprint = Sprint::get_current(app.clone(), self.sprint).await?;
		let content = sprint.status_text(app, false).await?;

		warn!(?content, "TODO: update announce");
		Ok(())
		// app.send_response(self.0).await.map(drop)
	}
}
