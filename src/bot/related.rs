use itertools::Itertools;
use miette::{miette, Context, IntoDiagnostic, Result};
use tracing::debug;
use twilight_model::application::{
	command::{Command, CommandType},
	interaction::{application_command::CommandData, Interaction},
};
use twilight_util::builder::command::{CommandBuilder, StringBuilder};
use url::Url;

use crate::bot::{
	action::CommandAck,
	context::{GenericResponse, GenericResponseData},
	utils::command::get_string,
};

use super::App;

#[tracing::instrument]
pub fn command() -> Result<Command> {
	CommandBuilder::new(
		"related",
		"Get related words. Uses https://relatedwords.org/".to_string(),
		CommandType::ChatInput,
	)
	.option(StringBuilder::new("word", "An english word").required(true))
	.validate()
	.into_diagnostic()
	.map(|cmd| cmd.build())
}

#[derive(Debug, serde::Deserialize)]
struct Related {
	word: String,
}

pub async fn on_command(
	app: App,
	interaction: &Interaction,
	command_data: &CommandData,
) -> Result<()> {
	let word = get_string(&command_data.options, "word").ok_or(miette!("needs a word"))?;
	app.do_action(CommandAck::new(&interaction)).await?;
	debug!(?word, "related arguments");

	let mut url = Url::parse("https://relatedwords.org/api/related").unwrap();
	url.query_pairs_mut().append_pair("term", &word);

	let terms: Vec<Related> = reqwest::get(url)
		.await
		.into_diagnostic()
		.wrap_err("GET relatedwords.org")?
		.json()
		.await
		.into_diagnostic()
		.wrap_err("decode json payload")?;

	app.send_response(GenericResponse::from_interaction(
		interaction,
		GenericResponseData {
			content: Some(format!("Terms related to **{word}**: {}. More at <https://relatedwords.org/relatedto/{word}>", terms.into_iter()
			.map(|term| term.word)
			.take(12).join(", "))),
			..Default::default()
		},
	))
	.await
	.map(drop)
}
