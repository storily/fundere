use chrono::Duration;
use miette::{miette, Context, Result};
use tracing::debug;
use twilight_model::{
	channel::message::component::{ButtonStyle, Button, Component},
	application::{
	interaction::Interaction,
}};

use crate::{
	bot::{
		context::{GenericResponse, GenericResponseData, Timer},
		utils::{
			action_row,
			time::{ChronoDurationExt, ChronoDateTimeExt},
		},
		App,
	},
	db::sprint::{Sprint, SprintStatus},
};

use super::{Action, ActionClass, Args, SprintStart, SprintStartWarning};

#[derive(Debug, Clone)]
pub struct SprintAnnounce {
	sprint: Box<Sprint>,
	response: Box<GenericResponse>,
}

impl SprintAnnounce {
	async fn prepare(app: App, sprint: &Sprint) -> Result<GenericResponseData> {
		if sprint.status >= SprintStatus::Announced {
			return Err(miette!("Bug: went to announce sprint but it was already"));
		}

		let components = action_row(vec![
			Component::Button(Button {
				custom_id: Some(format!("sprint:join:{}", sprint.id)),
				disabled: false,
				emoji: None,
				label: Some("Join".to_string()),
				style: ButtonStyle::Success,
				url: None,
			}),
			Component::Button(Button {
				custom_id: Some(format!("sprint:cancel:{}", sprint.id)),
				disabled: false,
				emoji: None,
				label: Some("Cancel".to_string()),
				style: ButtonStyle::Danger,
				url: None,
			}),
		]);

		sprint
			.update_status(app.clone(), SprintStatus::Announced)
			.await?;

		let warning_in = sprint.warning_in();
		if !warning_in.is_zero() {
			debug!(?warning_in, "set up sprint warn timer");
			app.send_timer(Timer::new_after(
				// UNWRAP: warning_in uses saturating_sub, will never be negative
				warning_in.to_std().unwrap(),
				SprintStartWarning::new(sprint),
			)?)
			.await?;
		}

		let starting_in = sprint.starting_in();
		debug!(?starting_in, "set up sprint start timer");
		app.send_timer(Timer::new_after(
			starting_in.positive_or(Duration::zero()).to_std().unwrap(),
			SprintStart::new(sprint),
		)?)
		.await?;

		Ok(GenericResponseData {
			content: Some(sprint.status_text(app, true).await?),
			components,
			..Default::default()
		})
	}

	#[tracing::instrument(name = "SprintAnnounce", skip(app, interaction))]
	pub async fn new(app: App, interaction: &Interaction, sprint: Sprint) -> Result<Action> {
		Ok(ActionClass::SprintAnnounce(Self {
			response: Box::new(GenericResponse::from_interaction(
				interaction,
				Self::prepare(app, &sprint).await?,
			).with_age(sprint.created_at.elapsed()?)),
			sprint: Box::new(sprint),
		})
		.into())
	}

	#[tracing::instrument(name = "SprintAnnounce::new_from_db", skip(app))]
	pub async fn new_from_db(app: App, sprint: Sprint) -> Result<Action> {
		Ok(ActionClass::SprintAnnounce(Self {
			response: Box::new(GenericResponse::from_sprint(
				&sprint,
				Self::prepare(app, &sprint).await?,
			)),
			sprint: Box::new(sprint),
		})
		.into())
	}

	pub async fn handle(self, Args { app, .. }: Args) -> Result<()> {
		let message = app.send_response(*self.response).await?;
		self.sprint
			.set_announce(app, (&message).try_into().wrap_err("convert message")?)
			.await?;
		Ok(())
	}
}
