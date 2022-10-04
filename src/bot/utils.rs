use std::iter::once;

use twilight_model::application::component::{ActionRow, Component};

pub mod command;
pub mod time;

pub fn action_row(components: Vec<Component>) -> impl Iterator<Item = Component> {
	once(Component::ActionRow(ActionRow { components }))
}
