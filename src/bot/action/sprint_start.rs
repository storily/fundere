use itertools::Itertools;
use miette::{miette, Result};
use std::time::Duration;
use tracing::debug;
use twilight_mention::Mention;
use twilight_model::channel::message::component::{Button, ButtonStyle, Component};
use uuid::Uuid;

use crate::{
	bot::{
		context::{GenericResponse, GenericResponseData, Timer},
		utils::{action_row, time::ChronoDateTimeExt},
	},
	db::sprint::{Sprint, SprintStatus},
};

use super::{Action, ActionClass, Args, SprintEnd, SprintEndWarning};

#[derive(Debug, Clone)]
pub struct SprintStart(Uuid);

impl SprintStart {
	#[tracing::instrument(name = "SprintStart")]
	pub fn new(sprint: &Sprint) -> Action {
		ActionClass::SprintStart(Box::new(Self(sprint.id))).into()
	}

	pub async fn handle(self, Args { app, .. }: Args) -> Result<()> {
		let sprint = Sprint::get_current(app.clone(), self.0).await?;
		if sprint.status >= SprintStatus::Started {
			return Err(miette!("Bug: went to start sprint but it was already"));
		}

		let Sprint { id, shortid, .. } = sprint;
		let duration = sprint.formatted_duration();

		let participant_list = sprint
			.participants(app.clone())
			.await?
			.iter()
			.map(|p| p.mention().to_string())
			.join(", ");

		let ending_abs = sprint.ending_at().discord_format('T');
		let ending_rel = sprint.ending_at().discord_format('R');

		let content = format!("⏱️ Sprint `{shortid}` is starting now for {duration}! (ending at {ending_abs} / {ending_rel}) // {participant_list}");
		// TODO: ding

		sprint
			.update_status(app.clone(), SprintStatus::Started)
			.await?;

		if let Ok(ending_in) = sprint.ending_in().to_std() {
			debug!("set up sprint end timer");
			app.send_timer(Timer::new_after(ending_in, SprintEnd::new(&sprint))?)
				.await?;

			debug!("set up sprint end warning timer");
			app.send_timer(Timer::new_after(
				ending_in.saturating_sub(Duration::from_secs(30)),
				SprintEndWarning::new(&sprint),
			)?)
			.await?;
		} else {
			return Err(miette!("sprint ended before it began???"));
		}

		app.send_response(GenericResponse::from_sprint(
			&sprint,
			GenericResponseData {
				content: Some(content),
				components: action_row(vec![
					Component::Button(Button {
						custom_id: Some(format!("sprint:join:{id}")),
						disabled: false,
						emoji: None,
						label: Some("Join late".to_string()),
						style: ButtonStyle::Secondary,
						url: None,
					}),
					Component::Button(Button {
						custom_id: Some(format!("sprint:start-words:{id}")),
						disabled: false,
						emoji: None,
						label: Some("Starting words".to_string()),
						style: ButtonStyle::Secondary,
						url: None,
					}),
				]),
				..Default::default()
			},
		))
		.await
		.map(drop)
	}
}
