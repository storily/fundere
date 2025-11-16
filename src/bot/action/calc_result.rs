use miette::{IntoDiagnostic, Result};
use twilight_model::application::interaction::Interaction;
use twilight_util::builder::embed::EmbedBuilder;

use crate::bot::context::{GenericResponse, GenericResponseData};

use super::{Action, ActionClass, Args};

#[derive(Debug, Clone)]
pub struct CalcResult(GenericResponse);

impl CalcResult {
	#[tracing::instrument(name = "CalcResult", skip(interaction))]
	pub fn new(
		interaction: &Interaction,
		input: &str,
		result: &str,
		public: bool,
	) -> Result<Action> {
		Ok(
			ActionClass::CalcResult(Box::new(Self(GenericResponse::from_interaction(
				interaction,
				GenericResponseData {
					ephemeral: !public,
					embeds: vec![
						EmbedBuilder::new()
							.color(0x00_00_FF)
							.description(input)
							.validate()
							.into_diagnostic()?
							.build(),
						EmbedBuilder::new()
							.color(0x00_FF_00)
							.description(result)
							.validate()
							.into_diagnostic()?
							.build(),
					],
					..Default::default()
				},
			))))
			.into(),
		)
	}

	pub async fn handle(self, Args { app, .. }: Args) -> Result<()> {
		app.send_response(self.0).await.map(drop)
	}
}
