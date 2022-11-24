use twilight_model::channel::message::component::{ActionRow, Component};

pub mod command;
pub mod time;

pub fn action_row(components: Vec<Component>) -> Vec<Component> {
	vec![Component::ActionRow(ActionRow { components })]
}
