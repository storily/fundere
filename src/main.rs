use futures_util::StreamExt;
use miette::{IntoDiagnostic, Result};
use tracing::info;
use twilight_gateway::Shard;

mod config;

#[tokio::main]
async fn main() -> Result<()> {
	tracing_subscriber::fmt::init();

	let config = config::Config::load("config.kdl").await?;
	dbg!(&config);

	let (shard, mut events) = Shard::new(config.discord.token, config.discord.intents.to_intent());

	shard.start().await.into_diagnostic()?;
	info!("Created shard");

	while let Some(event) = events.next().await {
		info!("Event: {event:?}");
	}

	Ok(())
}
