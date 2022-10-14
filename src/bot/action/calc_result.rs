use miette::{IntoDiagnostic, Result};
use twilight_model::{
	application::interaction::Interaction,
	channel::message::MessageFlags,
	http::interaction::{InteractionResponse, InteractionResponseType},
	id::{marker::InteractionMarker, Id},
};
use twilight_util::builder::{embed::EmbedBuilder, InteractionResponseDataBuilder};

use super::{Action, ActionClass, Args};

#[derive(Debug, Clone)]
pub struct CalcResult {
	pub id: Id<InteractionMarker>,
	pub token: String,
	pub input: String,
	pub result: String,
	pub public: bool,
}
impl CalcResult {
	#[tracing::instrument(name = "CalcResult", skip(interaction))]
	pub fn new(interaction: &Interaction, input: &str, result: &str, public: bool) -> Action {
		ActionClass::CalcResult(Self {
			id: interaction.id,
			token: interaction.token.clone(),
			input: input.to_string(),
			result: result.to_string(),
			public,
		})
		.into()
	}

	pub async fn handle(self, Args { app, .. }: Args) -> Result<()> {
		app.interaction_client()
			.create_response(
				self.id,
				&self.token,
				&InteractionResponse {
					kind: InteractionResponseType::ChannelMessageWithSource,
					data: Some(
						InteractionResponseDataBuilder::new()
							.flags(if self.public {
								MessageFlags::empty()
							} else {
								MessageFlags::EPHEMERAL
							})
							.embeds([
								EmbedBuilder::new()
									.color(0x00_00_FF)
									.description(self.input)
									.validate()
									.into_diagnostic()?
									.build(),
								EmbedBuilder::new()
									.color(0x00_FF_00)
									.description(self.result)
									.validate()
									.into_diagnostic()?
									.build(),
							])
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
