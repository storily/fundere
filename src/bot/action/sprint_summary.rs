use itertools::Itertools;
use miette::{IntoDiagnostic, Result};
use twilight_model::{
	application::interaction::Interaction,
	http::interaction::{InteractionResponse, InteractionResponseType},
	id::{marker::InteractionMarker, Id},
};
use twilight_util::builder::InteractionResponseDataBuilder;
use uuid::Uuid;

use crate::{
	bot::App,
	db::sprint::{Sprint, SprintStatus},
};

use super::{Action, ActionClass, Args};

#[derive(Debug, Clone)]
pub struct SprintSummary {
	pub id: Id<InteractionMarker>,
	pub token: String,
	pub sprint: Uuid,
	pub content: String,
}

impl SprintSummary {
	#[tracing::instrument(name = "SprintSummary", skip(app, interaction))]
	pub async fn new(app: App, interaction: &Interaction, sprint: Sprint) -> Result<Action> {
		let started_at = sprint
			.starting_at
			.with_timezone(&chrono_tz::Pacific::Auckland)
			.format("%H:%M:%S");

		let shortid = sprint.shortid;
		let duration = sprint.formatted_duration();
		let minutes = sprint.duration().num_minutes();

		let participants = sprint.participants(app.clone()).await?;
		let mut summaries = Vec::with_capacity(participants.len());
		for p in participants {
			let member = p.member.to_member(app.clone()).await?;
			let name = member.nick.unwrap_or_else(|| member.user.name);
			let words = p
				.words_end
				.and_then(|end| p.words_start.map(|start| end - start))
				.unwrap_or(0);
			let wpm = (words as f64) / (minutes as f64);
			summaries.push((name, words, wpm));
		}

		summaries.sort_by_key(|(_, w, _)| *w);
		let summary = summaries
			.into_iter()
			.map(|(name, words, wpm)| {
				format!("_{name}_: **{words}** words (**{wpm:.1}** words per minute)")
			})
			.join("\n");

		let content =
			format!("🧮 Sprint `{shortid}`, {duration}, started at {started_at}:\n{summary}");

		sprint
			.update_status(app.clone(), SprintStatus::Summaried)
			.await?;

		Ok(ActionClass::SprintSummary(Self {
			id: interaction.id,
			token: interaction.token.clone(),
			sprint: sprint.id,
			content,
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
		if as_followup {
			interaction_client
				.create_followup(&self.token)
				.content(&self.content)
				.into_diagnostic()?
				.exec()
				.await
				.into_diagnostic()
				.map(drop)
		} else {
			interaction_client
				.create_response(
					self.id,
					&self.token,
					&InteractionResponse {
						kind: InteractionResponseType::ChannelMessageWithSource,
						data: Some(
							InteractionResponseDataBuilder::new()
								.content(self.content)
								.build(),
						),
					},
				)
				.exec()
				.await
				.into_diagnostic()
				.map(drop)
		}
	}
}
