use std::iter::{once, repeat};

use miette::{GraphicalReportHandler, GraphicalTheme, IntoDiagnostic, Result, Report};
use twilight_http::client::InteractionClient;
use twilight_model::{
	channel::message::MessageFlags,
	http::interaction::{InteractionResponse, InteractionResponseType},
	id::{marker::InteractionMarker, Id}, application::interaction::Interaction,
};
use twilight_util::builder::{embed::EmbedBuilder, InteractionResponseDataBuilder};

#[derive(Debug, Clone)]
pub enum Action {
	CommandError(CommandError),
}

#[derive(Debug, Clone)]
pub struct CommandError {
	pub id: Id<InteractionMarker>,
	pub token: String,
	pub error: String,
}
impl CommandError {
pub async fn handle(
	interaction_client: &InteractionClient<'_>,
	data: CommandError,
) -> Result<()> {
	interaction_client
		.create_response(
			data.id,
			&data.token,
			&InteractionResponse {
				kind: InteractionResponseType::ChannelMessageWithSource,
				data: Some(
					InteractionResponseDataBuilder::new()
						.flags(MessageFlags::EPHEMERAL)
						.embeds(once(
							EmbedBuilder::new()
								.color(0xFF_00_00)
								.description(data.error)
								.validate()
								.into_diagnostic()?
								.build(),
						))
						.build(),
				),
			},
		)
		.exec()
		.await
		.into_diagnostic()?;
	Ok(())
}

pub fn from_report(interaction: &Interaction, err: Report) -> Result<Action> {
	let handler = GraphicalReportHandler::new_themed(GraphicalTheme::unicode_nocolor());
		let mut error = String::from("Error:\n```");
		handler
			.render_report(&mut error, err.as_ref())
			.into_diagnostic()?;
		error.extend(repeat('`').take(3));
		Ok(Action::CommandError(Self {
			id: interaction.id,
			token: interaction.token.clone(),
			error,
		}))
}
}
