use chrono::Duration;
use humantime::format_duration;
use miette::{miette, Result};
use tracing::debug;
use twilight_model::application::{
	component::{button::ButtonStyle, Button, Component},
	interaction::Interaction,
};

use crate::{
	bot::{
		context::{GenericResponse, GenericResponseData, MessageForm, Timer},
		utils::{
			action_row,
			time::{round_duration_to_seconds, ChronoDurationSaturatingSub},
		},
		App,
	},
	db::sprint::{Sprint, SprintStatus},
};

use super::{Action, ActionClass, Args, SprintStart, SprintWarning};

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

		let starting_at = sprint
			.starting_at
			.with_timezone(&chrono_tz::Pacific::Auckland)
			.format("%H:%M:%S");

		let shortid = sprint.shortid;
		let duration = sprint.formatted_duration();

		let starting_in = sprint.starting_in();
		let starting_in_disp = if starting_in <= Duration::zero() {
			"now".into()
		} else {
			format!(
				"in {}",
				format_duration(round_duration_to_seconds(starting_in))
			)
		};

		let content = format!(
			"⏱️  New sprint! `{shortid}` is starting {starting_in_disp} (at {starting_at}), going for {duration}."
		);

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
			debug!("set up sprint warn timer");
			app.send_timer(Timer::new_after(
				// UNWRAP: warning_in uses saturating_sub, will never be negative
				warning_in.to_std().unwrap(),
				SprintWarning::new(&sprint),
			)?)
			.await?;
		}

		debug!("set up sprint start timer");
		app.send_timer(Timer::new_after(
			starting_in.positive_or(Duration::zero()).to_std().unwrap(),
			SprintStart::new(&sprint),
		)?)
		.await?;

		Ok(GenericResponseData {
			content: Some(content),
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
			)),
			sprint: Box::new(sprint),
		})
		.into())
	}

	#[tracing::instrument(name = "SprintAnnounce::new_from_db", skip(app))]
	pub async fn new_from_db(app: App, sprint: Sprint) -> Result<Action> {
		Ok(ActionClass::SprintAnnounce(Self {
			response: Box::new(GenericResponse {
				channel: sprint.announce.map(|msg| msg.into()),
				interaction: None,
				token: Some(sprint.interaction_token.clone()),
				message: sprint.announce.map(MessageForm::Db),
				data: Self::prepare(app, &sprint).await?,
			}),
			sprint: Box::new(sprint),
		})
		.into())
	}

	pub async fn handle(self, Args { app, .. }: Args) -> Result<()> {
		let message = app.send_response(*self.response).await?;
		self.sprint
			.set_announce(app, (&message).try_into()?)
			.await?;
		Ok(())
	}
}
