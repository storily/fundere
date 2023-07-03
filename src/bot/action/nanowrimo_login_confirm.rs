
use miette::{Result};
use twilight_model::{
	channel::message::{ReactionType, component::{ButtonStyle, Button, Component}},
	application::{
	interaction::Interaction,
}};
use uuid::Uuid;

use crate::{
	bot::{
		context::{GenericResponse, GenericResponseData},
		utils::{
			action_row,
		},
	},
	db::member::Member,
};

use super::{Action, ActionClass, Args};

#[derive(Debug, Clone)]
pub struct NanowrimoLoginConfirm {
	response: Box<GenericResponse>,
}

impl NanowrimoLoginConfirm {
	#[tracing::instrument(name = "NanowrimoLoginConfirm", skip(interaction))]
	pub fn new(interaction: &Interaction, member: Member) -> Action {
		let member_uuid = Uuid::from(member);
		ActionClass::NanowrimoLoginConfirm(Self {
			response: Box::new(GenericResponse::from_interaction(
				interaction,
				GenericResponseData {
					content: Some("⚠️ Only provide your nanowrimo credentials if you trust this bot! It will in theory be able to do anything you can in your nanowrimo account. This particular bot uses your login to:\n- retrieve word counts and goals from your projects\n- update your word count\n\nAre you sure you want to login?".into()),
					components: action_row(vec![
						Component::Button(Button {
							custom_id: Some(format!("nanowrimo:login:{member_uuid}")),
							disabled: false,
							emoji: Some(ReactionType::Unicode { name: "🔐".into() }),
							label: Some("I'm sure, let's go".to_string()),
							style: ButtonStyle::Danger,
							url: None,
						}),
					]),
					ephemeral: true,
					..Default::default()
				},
			)),
		})
		.into()
	}

	pub async fn handle(self, Args { app, .. }: Args) -> Result<()> {
		app.send_response(*self.response).await.map(drop)
	}
}
