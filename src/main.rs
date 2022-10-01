use std::sync::Arc;

use futures_util::StreamExt;
use miette::{IntoDiagnostic, Result};
use tokio::task::spawn;
use tracing::{debug, info, warn};
use twilight_gateway::Shard;
use twilight_http::Client;
use twilight_model::{
	application::interaction::{Interaction, InteractionData},
	gateway::event::Event,
	id::Id,
};

use config::Config;

pub(crate) mod config;
mod sprint;

#[tokio::main]
async fn main() -> Result<()> {
	tracing_subscriber::fmt::init();

	let config = Config::load("config.kdl").await?;

	let listening = spawn(listen(config.clone()));
	let controlling = spawn(control(config));

	controlling.await.into_diagnostic()??;
	listening.await.into_diagnostic()??;

	Ok(())
}

async fn control(config: Arc<Config>) -> Result<()> {
	let client = Client::new(config.discord.token.clone());
	let application_id = Id::new(config.discord.app_id);

	let interaction_client = client.interaction(application_id);
	interaction_client
		.set_global_commands(&[sprint::command(config.clone())?])
		.exec()
		.await
		.into_diagnostic()?;

	Ok(())
}

async fn listen(config: Arc<Config>) -> Result<()> {
	let (shard, mut events) = Shard::new(
		config.discord.token.clone(),
		config.discord.intents.to_intent(),
	);

	shard.start().await.into_diagnostic()?;
	info!("Created shard");

	while let Some(event) = events.next().await {
		debug!("Event: {event:?}");
		// TODO: spawn off here
		match event {
			Event::InteractionCreate(ic) => handle_interaction(config.clone(), &ic.0).await?,
			_ => {}
		}
	}

	Ok(())
}

async fn handle_interaction(config: Arc<Config>, interaction: &Interaction) -> Result<()> {
	match &interaction.data {
		Some(InteractionData::ApplicationCommand(data)) => match data.name.as_str() {
			"sprint" => sprint::handle(config.clone(), interaction, &data).await?,
			cmd => warn!("unhandled command: {cmd}"),
		},
		Some(other) => warn!("unhandled interaction: {other:?}"),
		None => warn!("unspecified data for interaction"),
	}

	Ok(())
}
