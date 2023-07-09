use std::str::FromStr;

use itertools::Itertools;
use miette::{Context, IntoDiagnostic, Result};
use rand::Rng;
use tracing::{error, warn};
use twilight_model::application::{
	command::{Command, CommandType},
	interaction::{
		application_command::{CommandData, CommandDataOption, CommandOptionValue},
		Interaction,
	},
};
use twilight_util::builder::command::{
	CommandBuilder, IntegerBuilder, StringBuilder, SubCommandBuilder,
};

use crate::{
	bot::{
		action::CommandAck,
		context::{GenericResponse, GenericResponseData},
		utils::command::{get_integer, get_string},
		App,
	},
	error_ext::ErrorExt,
};

mod cards;

#[tracing::instrument]
pub fn command() -> Result<Command> {
	CommandBuilder::new(
		"random",
		"Get some random in your life".to_string(),
		CommandType::ChatInput,
	)
	.option(
		SubCommandBuilder::new("number", "Get a random number")
			.option(IntegerBuilder::new("min", "Minimum value (default: 0)"))
			.option(IntegerBuilder::new(
				"max",
				"Maximum value (default: as high as we can go)",
			))
			.option(IntegerBuilder::new(
				"count",
				"How many numbers to get (default: 1)",
			)),
	)
	.option(
		SubCommandBuilder::new("card-suit", "Get a random playing card suit")
			.option(
				StringBuilder::new("variant", "Which variant to use (default: all)").choices(vec![
					("Everything (default)", "all"),
					("English playing card suites", "english"),
					("French playing card suites", "french"),
					("German playing card suites", "german"),
					("Italian playing card suites", "italian"),
					("Spanish playing card suites", "spanish"),
					("Swiss playing card suites", "swiss"),
					("Tarot card suites", "tarot"),
					("Tarot nouveau card suites", "nouveau"),
					("Dashavatara Ganjifa (persia/india) card suites", "ganjifa"),
					("Moghul Ganjifa (persia/india) card suites", "moghul"),
					("Extended Hanafuda (japan/korea) card suites", "hanafuda"),
					(
						"Mahjong (china/japan/southeast asia) tile suites",
						"mahjong",
					),
				]),
			)
			.option(IntegerBuilder::new(
				"count",
				"How many suits to get (default: 1)",
			)),
	)
	.option(
		SubCommandBuilder::new("card-value", "Get a random playing card value")
			.option(
				StringBuilder::new("variant", "Which variant to use (default: all)").choices(vec![
					("Everything (default)", "all"),
					("Full 52-card deck values (ace, 2-10, jokers, JQK)", "full"),
					(
						"Extended 63-card deck values (ace, 2-13, joker, JQK)",
						"euchre",
					),
					("Cartomancy Tarot card values (1-10, JPKQK)", "tarot"),
					("Cartomancy Tarot Major Arcana", "arcana"),
					("Tarot nouveau 78-card values (1-10, JKQK)", "nouveau"),
					("Tarot nouveau honour values (1-21), plus fool", "honours"),
					("Tarot nouveau honour aspects, plus fool", "aspects"),
					(
						"Tarocco Siciliano 63-card values (1-10, MKQK)",
						"siciliano-v",
					),
					(
						"Tarocco Siciliano honour aspects, plus fugitive",
						"siciliano-h",
					),
					(
						"Tarocco Bolognese 62-card values (1-10, KKQK)",
						"bolognese-v",
					),
					("Tarocco Bolognese honour aspects, plus fool", "bolognese-h"),
					(
						"Tarocco Minchiate 97-card values (1-10, [MP]KQK)",
						"minchiate-v",
					),
					("Tarocco Minchiate honour aspects", "minchiate-h"),
					("Swiss 1JJ honour aspects", "1jj"),
					("Ganjifa values (1-10, vizier, king)", "ganjifa"),
					("Hanafuda values (hikari, tane, tanzaku, kasu)", "hanafuda"),
					("Mahjong values (1-9, winds, dragons, flowers)", "mahjong"),
				]),
			)
			.option(IntegerBuilder::new(
				"count",
				"How many values to get (default: 1)",
			)),
	)
	.option(
		SubCommandBuilder::new("card", "Get a random playing card or hand of cards")
			.option(
				StringBuilder::new("variant", "Which variant to use (default: all)").choices(vec![
					("Everything (default)", "all"),
					("Full 52-card english deck", "english"),
					("Full 52-card french deck", "french"),
					("Full 52-card german deck", "german"),
					("Full 52-card italian deck", "italian"),
					("Full 52-card spanish deck", "spanish"),
					("Full 52-card swiss deck", "swiss"),
					("Extended 63-card deck used to play Euchre or 500", "euchre"),
					("Cartomancy Tarot deck", "tarot"),
					("Tarot Nouveau 78-card deck", "nouveau"),
					("Tarocco Siciliano deck", "siciliano"),
					("Tarocco Bolognese deck", "bolognese"),
					("Tarocco Minchiate deck", "minchiate"),
					("Swiss 1JJ deck", "1jj"),
					("Dashavatara Ganjifa (persia/india) deck", "ganjifa"),
					("Moghul Ganjifa (persia/india) deck", "moghul"),
					("Extended Hanafuda (japan/korea) deck", "hanafuda"),
					("Mahjong (china/japan/southeast asia) tiles", "mahjong"),
				]),
			)
			.option(IntegerBuilder::new(
				"count",
				"How many cards to get (default: 1)",
			)),
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
	let subcmd = command_data.options.iter().find_map(|opt| {
		if let CommandOptionValue::SubCommand(ref sub) = opt.value {
			Some((opt.name.as_str(), sub.as_slice()))
		} else {
			None
		}
	});

	match subcmd {
		Some(("number", opts)) => number(app.clone(), interaction, opts)
			.await
			.wrap_err("command: number")?,
		Some(("card-suit", opts)) => suit(app.clone(), interaction, opts)
			.await
			.wrap_err("command: card-suit")?,
		Some(("card-value", opts)) => value(app.clone(), interaction, opts)
			.await
			.wrap_err("command: card-value")?,
		Some(("card", opts)) => card(app.clone(), interaction, opts)
			.await
			.wrap_err("command: card")?,
		Some((other, _)) => warn!("unhandled random subcommand: {other}"),
		_ => error!("unreachable bare random command"),
	}

	Ok(())
}

async fn number(app: App, interaction: &Interaction, options: &[CommandDataOption]) -> Result<()> {
	let count = get_integer(options, "count").unwrap_or(1);
	let min = get_integer(options, "min").unwrap_or(0);
	let max = get_integer(options, "max").unwrap_or(i64::MAX);
	app.do_action(CommandAck::new(&interaction))
		.await
		.log()
		.ok();

	let result = (0..count)
		.map(|_| rand::thread_rng().gen_range(min..=max))
		.map(|n| format!("**{}**", n))
		.join(", ");

	app.send_response(GenericResponse::from_interaction(
		interaction,
		GenericResponseData {
			content: Some(result),
			..Default::default()
		},
	))
	.await
	.map(drop)
}

async fn suit(app: App, interaction: &Interaction, options: &[CommandDataOption]) -> Result<()> {
	let count = get_integer(options, "count").unwrap_or(1);
	let variant = get_string(options, "variant").unwrap_or("all");
	let variant = cards::SuitVariant::from_str(&variant)?;
	app.do_action(CommandAck::new(&interaction))
		.await
		.log()
		.ok();

	let result = (0..count)
		.map(|_| variant.random())
		.map(|s| format!("**{}**", s))
		.join(", ");

	app.send_response(GenericResponse::from_interaction(
		interaction,
		GenericResponseData {
			content: Some(result),
			..Default::default()
		},
	))
	.await
	.map(drop)
}

async fn value(app: App, interaction: &Interaction, options: &[CommandDataOption]) -> Result<()> {
	let count = get_integer(options, "count").unwrap_or(1);
	let variant = get_string(options, "variant").unwrap_or("all");
	let variant = cards::ValueVariant::from_str(&variant)?;
	app.do_action(CommandAck::new(&interaction))
		.await
		.log()
		.ok();

	let result = (0..count)
		.map(|_| variant.random())
		.map(|s| format!("**{}**", s))
		.join(", ");

	app.send_response(GenericResponse::from_interaction(
		interaction,
		GenericResponseData {
			content: Some(result),
			..Default::default()
		},
	))
	.await
	.map(drop)
}

async fn card(app: App, interaction: &Interaction, options: &[CommandDataOption]) -> Result<()> {
	let count = get_integer(options, "count").unwrap_or(1);
	let variant = get_string(options, "variant").unwrap_or("all");
	let variant = cards::DeckVariant::from_str(&variant)?;
	app.do_action(CommandAck::new(&interaction))
		.await
		.log()
		.ok();

	let result = variant
		.hand(count as _)
		.into_iter()
		.map(|s| format!("**{}**", s))
		.join(", ");

	app.send_response(GenericResponse::from_interaction(
		interaction,
		GenericResponseData {
			content: Some(result),
			..Default::default()
		},
	))
	.await
	.map(drop)
}
