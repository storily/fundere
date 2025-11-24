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
pub struct TimezoneShow {
	response: Box<GenericResponse>,
}

impl TimezoneShow {
	#[tracing::instrument(name = "TimezoneShow", skip(interaction))]
	pub fn new(interaction: &Interaction, member: Member, timezone: String) -> Action {
		let member_uuid = Uuid::from(member);
		ActionClass::TimezoneShow(Box::new(Self {
			response: Box::new(GenericResponse::from_interaction(
				interaction,
				GenericResponseData {
					content: Some(format!(
						"🌍 Your current timezone is: **{}**\n\nThis timezone is used when you enter times for commands like `/sprint new`.",
						timezone
					)),
					components: action_row(vec![
						Component::Button(Button {
							custom_id: Some(format!("timezone:change:{member_uuid}")),
							disabled: false,
							emoji: None,
							label: Some("Change Timezone".to_string()),
							style: ButtonStyle::Primary,
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
