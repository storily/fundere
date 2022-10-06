use futures_util::{FutureExt, StreamExt};
use miette::{Context, IntoDiagnostic, Result};
use sqlx::postgres::PgPoolOptions;
use tokio::{
	sync::mpsc::{self, Receiver},
	task::{spawn, JoinSet},
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

use self::{action::CommandError, context::Timer};

mod action;
mod calc;
mod context;
mod sprint;
mod utils;

pub async fn start(config: Config) -> Result<()> {
	let pool = PgPoolOptions::new()
		.max_connections(5)
		.connect(config.db.url.as_deref().unwrap_or("foo"))
		.await
		.into_diagnostic()?;

	let (db, db_task) = config.db.connect().await?;

	let client = Client::new(config.discord.token.clone());
	let (control, actions) = mpsc::channel(config.internal.control_buffer);
	let (timer, timings) = mpsc::channel(config.internal.timer_buffer);
	let app = App::new(config, db, pool, client, control, timer);

	let querying = spawn(async { db_task.await.into_diagnostic() });
	let ticking = spawn(ticker(app.clone(), timings));
	let listening = spawn(listener(app.clone()));
	let controlling = spawn(controller(app, actions));

	controlling.await.into_diagnostic()??;
	info!("controller has finished, cancelling other tasks");
	listening.abort();
	ticking.abort();
	querying.abort();

	info!("show's over, goodbye");
	Ok(())
}

#[tracing::instrument(skip_all)]
async fn controller(app: App, mut actions: Receiver<action::Action>) -> Result<()> {
	let client = Client::new(app.config.discord.token.clone());
	let application_id = Id::new(app.config.discord.app_id);

	let interaction_client = client.interaction(application_id);

	info!("register commands: calc, sprint");
	interaction_client
		.set_global_commands(&[calc::command()?, sprint::command()?])
		.exec()
		.await
		.into_diagnostic()?;

	info!("wait for actions");
	while let Some(action) = actions.recv().await {
		debug!(?action, "action received at controller");
		use action::Action::*;
		let action_dbg = format!("action: {action:?}");
		match action {
			CalcResult(data) => data.handle(&interaction_client).await,
			CommandError(data) => data.handle(&interaction_client).await,
			SprintAnnounce(data) => data.handle(&interaction_client).await,
			SprintCancelled(data) => data.handle(&interaction_client).await,
			SprintJoined(data) => data.handle(&interaction_client).await,
			SprintLeft(data) => data.handle(&interaction_client).await,
			SprintStart(data) => data.handle(app.clone(), &interaction_client).await,
			SprintWarning(data) => data.handle(app.clone(), &interaction_client).await,
		}
		.wrap_err(action_dbg)
		.unwrap_or_else(|err| error!("{err:?}"));
	}

	Ok(())
}

#[tracing::instrument(skip_all)]
async fn listener(app: App) -> Result<()> {
	let (shard, mut events) = Shard::new(
		app.config.discord.token.clone(),
		app.config.discord.intents.to_intent(),
	);

	shard.start().await.into_diagnostic()?;
	info!("created shard");

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

#[tracing::instrument(skip_all)]
async fn ticker(app: App, mut timings: Receiver<Timer>) -> Result<()> {
	info!("initialise ticker");
	let mut timers = JoinSet::new();

	loop {
		if timers.is_empty() {
			debug!("timer queue is empty, watching channel only");
			if let Some(timer) = timings.recv().await {
				debug!(?timer, "timer received, enqueueing");
				timers.spawn(timer.to_sleep().map(|_| timer.payload));
			} else {
				debug!("queue is empty and channel is done, ticker exiting");
				break Ok(());
			}
		} else {
			debug!("watching channel and timers");
			tokio::select! {
				Some(timer) = timings.recv() => {
					debug!(?timer, "timer received, enqueueing");
					timers.spawn(timer.to_sleep().map(|_| timer.payload));
				}
				timer = timers.join_next() => {
					match timer {
						None => warn!("ticker timer set is empty, which shouldn't happen on this branch"),
						Some(Err(err)) => {
							error!(%err, "timer has failed, this should never happen");
						}
						Some(Ok(payload)) => {
							debug!(?payload, "timer has finished, executing");
							app.send_action(payload).await.unwrap_or_else(|err| {
								error!(%err, "sending timer payload failed");
							});
						}
					}
				}
				else => {
					debug!("ticker is finished");
					break Ok(());
				}
			}
		}
	}
}

#[tracing::instrument(skip_all)]
async fn handle_interaction(app: App, interaction: &Interaction) -> Result<()> {
	match &interaction.data {
		Some(InteractionData::ApplicationCommand(data)) => {
			handle_interaction_error(&app, interaction, async {
				debug!(command=?data.name, "handle slash command");
				match data.name.as_str() {
					"sprint" => sprint::on_command(app.clone(), interaction, &data)
						.await
						.wrap_err("command: sprint"),
					"calc" => calc::on_command(app.clone(), interaction, &data)
						.await
						.wrap_err("command: calc"),
					cmd => {
						warn!("unhandled command: {cmd}");
						Ok(())
					}
				}
			})
			.await?;
		}
		Some(InteractionData::MessageComponent(data)) => {
			handle_interaction_error(&app, interaction, async {
				let subids: Vec<&str> = data.custom_id.split(':').collect();
				debug!(?subids, "handle component message");
				match subids.first() {
					Some(&"sprint") => {
						sprint::on_component(app.clone(), interaction, &subids[1..], &data)
							.await
							.wrap_err("component: sprint")
					}
					Some(other) => {
						warn!("unhandled component action: {other:?}");
						Ok(())
					}
					None => Ok(()),
				}
			})
			.await?;
		}
		Some(other) => warn!("unhandled interaction: {other:?}"),
		None => warn!("unspecified data for interaction"),
	}

	Ok(())
}

async fn handle_interaction_error(
	app: &App,
	interaction: &Interaction,
	task: impl std::future::Future<Output = Result<()>>,
) -> Result<()> {
	if let Err(err) = task.await {
		app.send_action(CommandError::new(interaction, err)?).await
	} else {
		Ok(())
	}
}
