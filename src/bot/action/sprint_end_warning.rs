use miette::{miette, Result};
use uuid::Uuid;

use crate::{
	bot::{
		context::{GenericResponse, GenericResponseData},
		utils::time::ChronoDateTimeExt,
	},
	db::sprint::{Sprint, SprintStatus},
};

use super::{Action, ActionClass, Args};

#[derive(Debug, Clone)]
pub struct SprintEndWarning(Uuid);

impl SprintEndWarning {
	#[tracing::instrument(name = "SprintEndWarning")]
	pub fn new(sprint: &Sprint) -> Action {
		ActionClass::SprintEndWarning(Self(sprint.id)).into()
	}

	pub async fn handle(self, Args { app, .. }: Args) -> Result<()> {
		let sprint = Sprint::get_current(app.clone(), self.0).await?;
		if sprint.status >= SprintStatus::Ended {
			return Err(miette!(
				"Bug: went to warn sprint but it was already ended"
			));
		}

		let Sprint { shortid, .. } = sprint;
		let ending = sprint.ending_at().discord_format('R');

		let content = format!(
			"⏱️ Sprint `{shortid}` is ending soon: {ending}"
		);
		// TODO: dong

		app.send_response(GenericResponse::from_sprint(
			&sprint,
			GenericResponseData {
				content: Some(content),
				..Default::default()
			},
		))
		.await
		.map(drop)
	}
}
