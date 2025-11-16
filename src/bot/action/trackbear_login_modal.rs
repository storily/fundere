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
pub struct TrackbearLoginModal {
	pub id: Id<InteractionMarker>,
	pub token: String,
	pub member: Member,
}

impl TrackbearLoginModal {
	#[tracing::instrument(name = "TrackbearLoginModal", skip(interaction))]
	pub fn new(interaction: &Interaction, member: Member) -> Action {
		ActionClass::TrackbearLoginModal(Box::new(Self {
			id: interaction.id,
			token: interaction.token.clone(),
			member,
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
							.custom_id(format!("trackbear:login:{member_uuid}"))
							.title("TrackBear Login".to_string())
							.components(vec![Component::ActionRow(ActionRow {
								components: vec![Component::TextInput(TextInput {
									custom_id: "api_key".into(),
									label: "Your TrackBear API Key".into(),
									max_length: None,
									min_length: Some(1),
									placeholder: Some(
										"Get one from trackbear.app/account/api-keys/new".into(),
									),
									required: Some(true),
									style: TextInputStyle::Short,
									value: None,
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
