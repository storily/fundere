use miette::{IntoDiagnostic, Result};
use twilight_model::{
	channel::message::component::{TextInputStyle, ActionRow, Component, TextInput},
	application::{
		interaction::Interaction,
	},
	http::interaction::{InteractionResponse, InteractionResponseType},
	id::{marker::InteractionMarker, Id},
};
use twilight_util::builder::InteractionResponseDataBuilder;
use uuid::Uuid;

use crate::{
	db::{member::Member},
};

use super::{Action, ActionClass, Args};

#[derive(Debug, Clone)]
pub struct NanowrimoLoginModal {
	pub id: Id<InteractionMarker>,
	pub token: String,
	pub member: Member,
}

impl NanowrimoLoginModal {
	#[tracing::instrument(name = "NanowrimoLoginModal", skip(interaction))]
	pub fn new(interaction: &Interaction, member: Member) -> Action {
		ActionClass::NanowrimoLoginModal(Self {
			id: interaction.id,
			token: interaction.token.clone(),
			member,
		})
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
							.custom_id(format!("nanowrimo:login:{member_uuid}"))
							.title("NaNoWriMo Login".to_string())
							.components(vec![
                                Component::ActionRow(ActionRow{
									components: vec![
										Component::TextInput(TextInput {
											custom_id: "username".into(),
											label: "Your NaNoWriMo.org username".into(),
											max_length: None,
											min_length: Some(1),
											placeholder: None,
											required: Some(true),
											style: TextInputStyle::Short,
											value: None,
										})
									]
								}),
                                Component::ActionRow(ActionRow{
									components: vec![
										Component::TextInput(TextInput {
											custom_id: "password".into(),
											label: "Your NaNoWriMo.org password".into(),
											max_length: None,
											min_length: Some(1),
											placeholder: None,
											required: Some(true),
											style: TextInputStyle::Short,
											value: None,
										})
									]
								}),
							])
							.build(),
					),
				},
			)
			.await
			.into_diagnostic()?;

		Ok(())
	}
}
