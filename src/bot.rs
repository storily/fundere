use futures_util::StreamExt;
use miette::{Context, IntoDiagnostic, Result};
use sqlx::postgres::PgPoolOptions;
use tokio::{
	sync::mpsc::{self, Receiver},
	task::spawn,
};
use tracing::{debug, error, info, warn};
use twilight_gateway::Shard;
use twilight_http::Client;
use twilight_model::{
	application::interaction::{Interaction, InteractionData},
	gateway::event::Event,
	id::Id,
};

use crate::config::Config;
pub(crate) use context::App;

use self::action::CommandError;

mod action;
mod context;
mod sprint;

pub async fn start(config: Config) -> Result<()> {
	let pool = PgPoolOptions::new()
		.max_connections(5)
		.connect(&config.db.url)
		.await
		.into_diagnostic()?;

	let (control, actions) = mpsc::channel(config.internal.control_buffer);
	let app = App::new(config, pool, control);

	let listening = spawn(listener(app.clone()));
	let controlling = spawn(controller(app, actions));

	controlling.await.into_diagnostic()??;
	info!("controller has finished, cancelling other tasks");
	listening.abort();

	info!("show's over, goodbye");
	Ok(())
}

async fn controller(app: App, mut actions: Receiver<action::Action>) -> Result<()> {
	let client = Client::new(app.config.discord.token.clone());
	let application_id = Id::new(app.config.discord.app_id);

	let interaction_client = client.interaction(application_id);
	interaction_client
		.set_global_commands(&[sprint::command(app.clone())?])
		.exec()
		.await
		.into_diagnostic()?;

	while let Some(action) = actions.recv().await {
		debug!(?action, "action received at controller");
		use action::Action::*;
		let action_dbg = format!("action: {action:?}");
		match action {
			CommandError(data) => data.handle(&interaction_client).await,
			SprintAnnounce(data) => data.handle(&interaction_client).await,
		}
		.wrap_err(action_dbg)?;
	}

	Ok(())
}

async fn listener(app: App) -> Result<()> {
	let (shard, mut events) = Shard::new(
		app.config.discord.token.clone(),
		app.config.discord.intents.to_intent(),
	);

	shard.start().await.into_diagnostic()?;
	info!("Created shard");

	while let Some(event) = events.next().await {
		debug!(?event, "spawning off to handle event");

		let app = app.clone();
		spawn(async move {
			match event {
				Event::InteractionCreate(ic) => handle_interaction(app.clone(), &ic.0)
					.await
					.wrap_err("event: interaction-create"),
				_ => Ok(()),
			}
			.unwrap_or_else(|err| error!("{err:?}"))
		});
	}

	Ok(())
}

async fn handle_interaction(app: App, interaction: &Interaction) -> Result<()> {
	match &interaction.data {
		Some(InteractionData::ApplicationCommand(data)) => {
			if let Err(err) = match data.name.as_str() {
				"sprint" => sprint::handle(app.clone(), interaction, &data)
					.await
					.wrap_err("command: sprint"),
				cmd => {
					warn!("unhandled command: {cmd}");
					Ok(())
				}
			} {
				app.send_action(CommandError::new(interaction, err)?)
					.await?;
			}
		}
		Some(other) => warn!("unhandled interaction: {other:?}"),
		None => warn!("unspecified data for interaction"),
	}

	Ok(())
}
