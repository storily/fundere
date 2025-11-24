use miette::{IntoDiagnostic, Result};
use twilight_model::{
	application::interaction::Interaction,
	channel::message::component::{ActionRow, Component, TextInput, TextInputStyle},
	http::interaction::{InteractionResponse, InteractionResponseType},
	id::{marker::InteractionMarker, Id},
};
use twilight_util::builder::InteractionResponseDataBuilder;
use uuid::Uuid;

use crate::db::member::Member;

use super::{Action, ActionClass, Args};

#[derive(Debug, Clone)]
pub struct TimezoneModal {
	pub id: Id<InteractionMarker>,
	pub token: String,
	pub member: Member,
	pub current_timezone: String,
}

impl TimezoneModal {
	#[tracing::instrument(name = "TimezoneModal", skip(interaction))]
	pub fn new(interaction: &Interaction, member: Member, current_timezone: String) -> Action {
		ActionClass::TimezoneModal(Box::new(Self {
			id: interaction.id,
			token: interaction.token.clone(),
			member,
			current_timezone,
		}))
		.into()
	}

	pub async fn handle(self, Args { app, .. }: Args) -> Result<()> {
		let member_uuid = Uuid::from(self.member);
		app.interaction_client()
			.create_response(
				self.id,
				&self.token,
				&InteractionResponse {
					kind: InteractionResponseType::Modal,
					data: Some(
						InteractionResponseDataBuilder::new()
							.custom_id(format!("timezone:set:{member_uuid}"))
							.title("Set your timezone".to_string())
							.content("This will be used for /sprint new. List here: https://en.wikipedia.org/wiki/List_of_tz_database_time_zones#List")
							.components(vec![Component::ActionRow(ActionRow {
								components: vec![Component::TextInput(TextInput {
									custom_id: "timezone".into(),
									label: "Timezone (TZ / IANA format)".into(),
									max_length: Some(100),
									min_length: Some(1),
									placeholder: Some(
										"e.g. America/New_York, Europe/London, Pacific/Auckland"
											.into(),
									),
									required: Some(true),
									style: TextInputStyle::Short,
									value: Some(self.current_timezone),
								})],
							})])
							.build(),
					),
				},
			)
			.await
			.into_diagnostic()?;

		Ok(())
	}
}
