use miette::Result;
use twilight_model::{
	application::interaction::Interaction,
	channel::message::component::{Button, ButtonStyle, Component},
};
use uuid::Uuid;

use crate::{
	bot::{
		context::{GenericResponse, GenericResponseData},
		utils::action_row,
	},
	db::member::Member,
};

use super::{Action, ActionClass, Args};

#[derive(Debug, Clone)]
pub struct TrackbearLoginConfirm {
	response: Box<GenericResponse>,
}

impl TrackbearLoginConfirm {
	#[tracing::instrument(name = "TrackbearLoginConfirm", skip(interaction))]
	pub fn new(interaction: &Interaction, member: Member) -> Action {
		let member_uuid = Uuid::from(member);
		ActionClass::TrackbearLoginConfirm(Box::new(Self {
			response: Box::new(GenericResponse::from_interaction(
				interaction,
				GenericResponseData {
					content: Some("⚠️ Only provide your TrackBear API key if you trust this bot! It will be able to access your TrackBear account. This particular bot uses your API key to:\n- retrieve word counts and goals from your projects\n- update your word count\n\nYou can create an API key at: https://trackbear.app/account/api-keys/new\n\nAre you sure you want to login?".into()),
					components: action_row(vec![
						Component::Button(Button {
							custom_id: Some(format!("trackbear:login:{member_uuid}")),
							disabled: false,
							emoji: None,
							label: Some("🔐 I'm sure, let's go".to_string()),
							style: ButtonStyle::Danger,
							url: None,
							sku_id: None,
						}),
					]),
					ephemeral: true,
					..Default::default()
				},
			)),
		}))
		.into()
	}

	pub async fn handle(self, Args { app, .. }: Args) -> Result<()> {
		app.send_response(*self.response).await.map(drop)
	}
}
