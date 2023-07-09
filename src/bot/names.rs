use itertools::Itertools;
use miette::{IntoDiagnostic, Result};

use tracing::{debug, warn};
use twilight_model::application::{
	command::{Command, CommandType},
	interaction::{application_command::CommandData, Interaction},
};
use twilight_util::builder::command::{CommandBuilder, IntegerBuilder, StringBuilder};

use crate::{
	bot::{
		action::CommandAck,
		context::{GenericResponse, GenericResponseData},
		utils::command::{get_integer, get_string},
		App,
	},
	error_ext::ErrorExt,
};

#[tracing::instrument]
pub fn command() -> Result<Command> {
	CommandBuilder::new("names", "Generate names!", CommandType::ChatInput)
		.option(
			IntegerBuilder::new("count", "How many names you want (max 100)")
				.min_value(1)
				.max_value(100),
		)
		.option(
			StringBuilder::new("gender", "Gender of the names").choices(strung(&[
				("enby", "enby"),
				("female", "female"),
				("male", "male"),
			])),
		)
		.option(
			StringBuilder::new("part", "Surnames or given names (defaults to full names)").choices(
				strung(&[("surname", "surname"), ("given", "given"), ("full", "full")]),
			),
		)
		.option(
			StringBuilder::new(
				"kind",
				"Name ethnicity/origin, or area of the world where names are used",
			)
			.choices(strung(&[
				// uncomment where there's more names in there
				// ("mideast", "mideast"),
				// ("easteuro", "easteuro"),
				// ("amerindian", "amerindian"),
				// ("pacific", "pacific"),
				("maori", "maori"),
				("french", "french"),
				("aboriginal", "aboriginal"),
				("latin", "latin"),
				("indian", "indian"),
				("afram", "afram"),
				("english", "english"),
			])),
		)
		.option(
			StringBuilder::new(
				"common",
				"How common the names should be (for more control use frequency)",
			)
			.choices(strung(&[("common", "common"), ("rare", "rare")])),
		)
		.option(
			IntegerBuilder::new(
				"frequency",
				"How common the names should at most be (1-100%)",
			)
			.min_value(1)
			.max_value(100),
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
	let Some(ref nominare) = app.nominare else {
		return app.send_response(GenericResponse::from_interaction(
			interaction,
			GenericResponseData {
				content: Some("Sorry! /names is not activated for this bot".to_string()),
				ephemeral: true,
				..Default::default()
			},
		))
		.await
		.map(drop);
	};

	let query = [
		get_integer(&command_data.options, "count").map(|n| n.to_string()),
		get_string(&command_data.options, "gender").map(|n| n.to_string()),
		get_string(&command_data.options, "part").map(|n| n.to_string()),
		get_string(&command_data.options, "kind").map(|n| n.to_string()),
		get_string(&command_data.options, "common").map(|n| n.to_string()),
		get_integer(&command_data.options, "frequency").map(|n| format!("{n}%")),
	]
	.iter()
	.filter_map(|x| x.as_ref())
	.join(" ");
	debug!(?query, "nominare: query");
	app.do_action(CommandAck::new(&interaction))
		.await
		.log()
		.ok();

	let mut names = nominare.search(&query).await.into_diagnostic()?;
	debug!(?query, ?names, "nominare: results");

	let mut response: String = names.iter().map(|name| name.to_string()).join(", ");

	while response.len() > 2000 {
		warn!(responses=?response.len(), "response too long, dropping a name");
		names.pop();
		response = names.iter().map(|name| name.to_string()).join(", ");
	}

	app.send_response(GenericResponse::from_interaction(
		interaction,
		GenericResponseData {
			content: Some(response),
			..Default::default()
		},
	))
	.await
	.map(drop)
}

fn strung(pairs: &[(&str, &str)]) -> Vec<(String, String)> {
	pairs
		.iter()
		.map(|(k, v)| (k.to_string(), v.to_string()))
		.collect()
}
