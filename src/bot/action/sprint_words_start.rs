use miette::{miette, IntoDiagnostic, Result};
use twilight_http::client::InteractionClient;
use twilight_model::{
	application::{
		component::{text_input::TextInputStyle, Component, TextInput},
		interaction::Interaction,
	},
	http::interaction::{InteractionResponse, InteractionResponseType},
	id::{marker::InteractionMarker, Id},
};
use twilight_util::builder::InteractionResponseDataBuilder;
use uuid::Uuid;

use crate::{
	bot::{utils::action_row, App},
	db::sprint::{Sprint, SprintStatus},
};

use super::Action;

#[derive(Debug, Clone)]
pub struct SprintWordsStart {
	pub id: Id<InteractionMarker>,
	pub token: String,
	pub sprint: Uuid,
}

impl SprintWordsStart {
	#[tracing::instrument(name = "SprintWordsStart", skip(interaction))]
	pub fn new(interaction: &Interaction, sprint: Uuid) -> Action {
		Action::SprintWordsStart(Self {
			id: interaction.id,
			token: interaction.token.clone(),
			sprint,
		})
	}

	pub async fn handle(self, app: App, interaction_client: &InteractionClient<'_>) -> Result<()> {
		let sprint = Sprint::get(app.clone(), self.sprint).await?;
		if sprint.is_cancelled() {
			return Err(miette!("Can't set starting words on a cancelled sprint."));
		}
		if sprint.status >= SprintStatus::Summaried {
			return Err(miette!(
				"Can't set starting words on a sprint that's already been finalised."
			));
		}

		let Sprint { id, shortid, .. } = sprint;

		interaction_client
			.create_response(
				self.id,
				&self.token,
				&InteractionResponse {
					kind: InteractionResponseType::Modal,
					data: Some(
						InteractionResponseDataBuilder::new()
							.custom_id(format!("sprint:set-words:start:{id}"))
							.title(format!("Starting words for sprint {shortid}"))
							.components(action_row(vec![Component::TextInput(TextInput {
								custom_id: "words".into(),
								label: "How many words are you starting with?".into(),
								max_length: Some(20),
								min_length: Some(1),
								placeholder: None,
								required: Some(true),
								style: TextInputStyle::Short,
								value: Some("0".into()),
							})]))
							.build(),
					),
				},
			)
			.exec()
			.await
			.into_diagnostic()?;

		Ok(())
	}
}
