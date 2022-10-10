use chrono::Duration;
use humantime::format_duration;
use miette::{miette, IntoDiagnostic, Result};
use tracing::debug;
use twilight_model::{
	application::{
		component::{button::ButtonStyle, Button, Component},
		interaction::Interaction,
	},
	http::interaction::{InteractionResponse, InteractionResponseType},
	id::{marker::InteractionMarker, Id},
};
use twilight_util::builder::InteractionResponseDataBuilder;
use uuid::Uuid;

use crate::{
	bot::{
		context::Timer,
		utils::{action_row, time::ChronoDurationSaturatingSub},
		App,
	},
	db::sprint::{Sprint, SprintStatus},
};

use super::{Action, ActionClass, Args, SprintStart, SprintWarning};

#[derive(Debug, Clone)]
pub struct SprintAnnounce {
	pub id: Option<Id<InteractionMarker>>,
	pub token: String,
	pub sprint: Uuid,
	pub content: String,
}

impl SprintAnnounce {
	async fn prepare(app: App, sprint: Sprint) -> Result<String> {
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
				format_duration(
					starting_in
						.to_std()
						.expect("starting_in is always above zero"),
				)
			)
		};

		let content = format!(
			"⏱️  New sprint! `{shortid}` is starting {starting_in_disp} (at {starting_at}), going for {duration}."
		);

		sprint
			.update_status(app.clone(), SprintStatus::Announced)
			.await?;

		let warning_in = starting_in.saturating_sub_std(Duration::seconds(30));
		if !warning_in.is_zero() {
			debug!("set up sprint warn timer");
			app.send_timer(Timer::new_after(warning_in, SprintWarning::new(&sprint))?)
				.await?;
		}

		debug!("set up sprint start timer");
		app.send_timer(Timer::new_after(
			starting_in.positive_or(Duration::zero()).to_std().unwrap(),
			SprintStart::new(&sprint),
		)?)
		.await?;

		Ok(content)
	}

	#[tracing::instrument(name = "SprintAnnounce::with_interaction", skip(app, interaction))]
	pub async fn with_interaction(
		app: App,
		interaction: &Interaction,
		sprint: Sprint,
	) -> Result<Action> {
		Ok(ActionClass::SprintAnnounce(Self {
			id: Some(interaction.id),
			token: interaction.token.clone(),
			sprint: sprint.id,
			content: Self::prepare(app, sprint).await?,
		})
		.into())
	}

	#[tracing::instrument(name = "SprintAnnounce::with_db_token", skip(app))]
	pub async fn with_db_token(app: App, sprint: Sprint) -> Result<Action> {
		Ok(ActionClass::SprintAnnounce(Self {
			id: None,
			token: sprint.interaction_token.clone(),
			sprint: sprint.id,
			content: Self::prepare(app, sprint).await?,
		})
		.into())
	}

	pub async fn handle(
		self,
		Args {
			interaction_client,
			as_followup,
			..
		}: Args<'_>,
	) -> Result<()> {
		let sprint_id = self.sprint;
		let components = action_row(vec![
			Component::Button(Button {
				custom_id: Some(format!("sprint:join:{sprint_id}")),
				disabled: false,
				emoji: None,
				label: Some("Join".to_string()),
				style: ButtonStyle::Success,
				url: None,
			}),
			Component::Button(Button {
				custom_id: Some(format!("sprint:cancel:{sprint_id}")),
				disabled: false,
				emoji: None,
				label: Some("Cancel".to_string()),
				style: ButtonStyle::Danger,
				url: None,
			}),
		]);

		if as_followup {
			interaction_client
				.create_followup(&self.token)
				.content(&self.content)
				.into_diagnostic()?
				.components(&components)
				.into_diagnostic()?
				.exec()
				.await
				.into_diagnostic()
				.map(drop)
		} else if let Some(interaction_id) = self.id {
			interaction_client
				.create_response(
					interaction_id,
					&self.token,
					&InteractionResponse {
						kind: InteractionResponseType::ChannelMessageWithSource,
						data: Some(
							InteractionResponseDataBuilder::new()
								.content(self.content)
								.components(components)
								.build(),
						),
					},
				)
				.exec()
				.await
				.into_diagnostic()
				.map(drop)
		} else {
			Err(miette!(
				"cannot handle interaction with no id or not a followup"
			))
		}
	}
}
