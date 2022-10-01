use futures_util::StreamExt;
use miette::{IntoDiagnostic, Result};
use sqlx::{postgres::PgPoolOptions};
use tokio::task::spawn;
use tracing::{debug, info, warn};
use twilight_gateway::Shard;
use twilight_http::Client;
use twilight_model::{
	application::interaction::{Interaction, InteractionData},
	gateway::event::Event,
	id::Id,
};

pub(crate) use context::App;

pub(crate) mod config;
mod context;
mod sprint;

#[tokio::main]
async fn main() -> Result<()> {
	tracing_subscriber::fmt::init();

	let config = config::Config::load("config.kdl").await?;
	let pool = PgPoolOptions::new()
		.max_connections(5)
		.connect(&config.db.url)
		.await.into_diagnostic()?;
	let app = App::new(config, pool);

	let listening = spawn(listen(app.clone()));
	let controlling = spawn(control(app));

	controlling.await.into_diagnostic()??;
	listening.await.into_diagnostic()??;

	Ok(())
}

async fn control(app: App) -> Result<()> {
	let client = Client::new(app.config.discord.token.clone());
	let application_id = Id::new(app.config.discord.app_id);

	let interaction_client = client.interaction(application_id);
	interaction_client
		.set_global_commands(&[sprint::command(app.clone())?])
		.exec()
		.await
		.into_diagnostic()?;

	Ok(())
}

async fn listen(app: App) -> Result<()> {
	let (shard, mut events) = Shard::new(
		app.config.discord.token.clone(),
		app.config.discord.intents.to_intent(),
	);

	shard.start().await.into_diagnostic()?;
	info!("Created shard");

	while let Some(event) = events.next().await {
		debug!("Event: {event:?}");
		// TODO: spawn off here
		match event {
			Event::InteractionCreate(ic) => handle_interaction(app.clone(), &ic.0).await?,
			_ => {}
		}
	}

	Ok(())
}

async fn handle_interaction(app: App, interaction: &Interaction) -> Result<()> {
	match &interaction.data {
		Some(InteractionData::ApplicationCommand(data)) => match data.name.as_str() {
			"sprint" => sprint::handle(app.clone(), interaction, &data).await?,
			cmd => warn!("unhandled command: {cmd}"),
		},
		Some(other) => warn!("unhandled interaction: {other:?}"),
		None => warn!("unspecified data for interaction"),
	}

	Ok(())
}
