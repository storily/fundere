use itertools::Itertools;
use miette::{miette, IntoDiagnostic, Result};
use rand::{distributions::Uniform, Rng};
use regex::Regex;
use tracing::debug;
use twilight_model::application::{
	command::{Command, CommandType},
	interaction::{application_command::CommandData, Interaction},
};
use twilight_util::builder::command::{CommandBuilder, IntegerBuilder, StringBuilder};

use crate::bot::{
	context::{GenericResponse, GenericResponseData},
	utils::command::{get_integer, get_string},
};

use super::App;

#[tracing::instrument]
pub fn command() -> Result<Command> {
	CommandBuilder::new(
		"choose",
		"Choose between some items".to_string(),
		CommandType::ChatInput,
	)
	.option(
		StringBuilder::new(
			"items",
			"One or more items, separated by the word \"or\" or commas",
		)
		.required(true),
	)
	.option(
		IntegerBuilder::new(
			"count",
			"Number of items to choose (default: 1). Ignored if only one item is given.",
		)
		.min_value(1),
	)
	.validate()
	.into_diagnostic()
	.map(|cmd| cmd.build())
}

pub async fn on_command(
	app: App,
	interaction: &Interaction,
	command_data: &CommandData,
) -> Result<()> {
	let mut count = get_integer(&command_data.options, "count").unwrap_or(1);
	let items_str =
		get_string(&command_data.options, "items").ok_or(miette!("need at least one item"))?;
	debug!(items=?items_str, ?count, "choose arguments");

	let or = Regex::new(r"(?i)\s+or\s+").unwrap();
	let mut items: Vec<&str> = or.split(items_str).map(|i| i.trim()).collect();

	if items.len() == 1 && items_str.contains(',') {
		let comma = Regex::new(r"(?i),\s*").unwrap();
		items = comma.split(items_str).map(|i| i.trim()).collect();
	}

	let single = items.len() == 1;

	if let Some(write) = items.iter().find(|i| i.eq_ignore_ascii_case("write")) {
		items.push(write.clone());
	}

	if let Some(sprint) = items.iter().find(|i| i.eq_ignore_ascii_case("sprint")) {
		items.push(sprint.clone());
	}

	if single {
		count = 1;

		// this is so we can respect the weighing even if a single item is given
		items = vec!["yes"; items.len()];
		items.push("no");
	}

	debug!(items=?items, ?count, "choosing");
	let result = rand::thread_rng()
		.sample_iter(Uniform::from(0..items.len()))
		.take(count as _)
		.filter_map(|i| items.get(i))
		.map(|item| format!("**{item}**"))
		.join(", ");

	app.send_response(GenericResponse::from_interaction(
		interaction,
		GenericResponseData {
			content: Some(format!("{items_str}? {result}")),
			..Default::default()
		},
	))
	.await
	.map(drop)
}
