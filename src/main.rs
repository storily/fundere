use std::sync::Arc;

use config::DiscordConfig;
use futures_util::StreamExt;
use miette::{IntoDiagnostic, Result};
use tokio::task::spawn;
use tracing::info;
use twilight_gateway::Shard;
use twilight_http::Client;
use twilight_model::application::command::CommandType;
use twilight_model::id::Id;
use twilight_util::builder::command::{
	CommandBuilder, IntegerBuilder, StringBuilder, SubCommandBuilder,
};

mod config;

#[tokio::main]
async fn main() -> Result<()> {
	tracing_subscriber::fmt::init();

	let config = config::Config::load("config.kdl").await?;

	let discord_config = Arc::new(config.discord);
	let listening = spawn(listen(discord_config.clone()));
	let controlling = spawn(control(discord_config));

	controlling.await.into_diagnostic()??;
	listening.await.into_diagnostic()??;

	Ok(())
}

async fn control(config: Arc<DiscordConfig>) -> Result<()> {
	let command = CommandBuilder::new(
		"sprint",
		"Experimental new-gen wordwar/sprint command",
		CommandType::ChatInput,
	)
	.option(
		SubCommandBuilder::new("start", "Create a new sprint").option(StringBuilder::new(
			"when",
			"When to start the sprint, either in clock time (08:30), or in relative time (+15m)",
		).required(true)).option(IntegerBuilder::new("duration", "Duration of the sprint in minutes (defaults to 15)")),
	).validate().into_diagnostic()?.build();

	let client = Client::new(config.token.clone());
	let application_id = Id::new(config.app_id);

	let interaction_client = client.interaction(application_id);
	interaction_client
		.set_global_commands(&[command])
		.exec()
		.await
		.into_diagnostic()?;

	Ok(())
}

async fn listen(config: Arc<DiscordConfig>) -> Result<()> {
	let (shard, mut events) = Shard::new(config.token.clone(), config.intents.to_intent());

	shard.start().await.into_diagnostic()?;
	info!("Created shard");

	while let Some(event) = events.next().await {
		info!("Event: {event:?}");
	}

	Ok(())
}
