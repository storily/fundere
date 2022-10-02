use std::iter::once;

use miette::{IntoDiagnostic, Result};
use twilight_http::client::InteractionClient;
use twilight_model::{
	channel::message::MessageFlags,
	http::interaction::{InteractionResponse, InteractionResponseType},
	id::{marker::InteractionMarker, Id},
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
pub async fn handle_command_error(
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
