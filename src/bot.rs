use futures_util::FutureExt;
use miette::{Context, IntoDiagnostic, Report, Result};
use tokio::{
	signal,
	sync::mpsc::{self, Receiver},
	task::{spawn, JoinSet},
};
use tracing::{debug, error, info, warn};
use twilight_gateway::{EventTypeFlags, Shard, ShardId, StreamExt};
use twilight_model::{
	application::interaction::{Interaction, InteractionData},
	gateway::event::Event,
};

use crate::config::Config;
pub(crate) use context::App;

use self::{action::CommandError, context::Timer};

pub mod action;
pub mod calc;
pub mod choose;
pub mod context;
pub mod debug;
pub mod names;
pub mod random;
pub mod related;
pub mod sprint;
pub mod trackbear;
pub mod utils;
pub mod words;

pub async fn start(config: Config) -> Result<()> {
	let (db, db_task) = config.db.connect().await?;

	let (timer, timings) = mpsc::channel(config.internal.timer_buffer);
	let app = App::new(config, db, timer);

	let querying = spawn(async {
		info!("starting db worker");
		db_task.await.into_diagnostic()
	});

	{
		let interaction_client = app.interaction_client();

		info!("register commands");
		interaction_client
			.set_global_commands(&[
				calc::command()?,
				choose::command()?,
				debug::command()?,
				names::command()?,
				trackbear::command()?,
				random::command()?,
				related::command()?,
				sprint::command()?,
				words::command()?,
			])
			.await
			.into_diagnostic()?;
	}

	let ticking = spawn(ticker(app.clone(), timings));
	let listening = spawn(listener(app.clone()));

	let initing = spawn(async {
		sprint::load_from_db(app).await?;
		Ok::<_, Report>(())
	});

	initing.await.into_diagnostic()??;
	info!("init has finished, good sailing!");

	signal::ctrl_c().await.into_diagnostic()?;
	info!("ctrl-c received, shutting down");
	listening.abort();
	ticking.abort();
	querying.abort();

	info!("show's over, goodbye");
	Ok(())
}

#[tracing::instrument(skip_all)]
async fn listener(app: App) -> Result<()> {
	let mut shard = Shard::new(
		ShardId::ONE,
		app.config.discord.token.clone(),
		app.config.discord.intents.to_intent(),
	);

	info!("created shard");

	loop {
		let event = match shard.next_event(EventTypeFlags::all()).await {
			Some(Ok(event)) => event,
			Some(Err(err)) => {
				error!(?err, "error receiving event");
				continue;
			}
			None => {
				info!("shard stream ended");
				break;
			}
		};

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

	info!("listener exiting");
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
							info!(?payload, "timer has finished, executing");
							app.do_action(payload).await.unwrap_or_else(|err| error!("{err:?}"));
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
			handle_interaction_error(app.clone(), interaction, async {
				info!(command=?data.name, "handle slash command");
				match data.name.as_str() {
					"calc" => calc::on_command(app.clone(), interaction, data)
						.await
						.wrap_err("command: calc"),
					"choose" => choose::on_command(app.clone(), interaction, data)
						.await
						.wrap_err("command: choose"),
					"debug" => debug::on_command(app.clone(), interaction, data)
						.await
						.wrap_err("command: debug"),
					"names" => names::on_command(app.clone(), interaction, data)
						.await
						.wrap_err("command: names"),
					"trackbear" => trackbear::on_command(app.clone(), interaction, data)
						.await
						.wrap_err("command: trackbear"),
					"random" => random::on_command(app.clone(), interaction, data)
						.await
						.wrap_err("command: random"),
					"related" => related::on_command(app.clone(), interaction, data)
						.await
						.wrap_err("command: related"),
					"sprint" => sprint::on_command(app.clone(), interaction, data)
						.await
						.wrap_err("command: sprint"),
					"words" => words::on_command(app.clone(), interaction, data)
						.await
						.wrap_err("command: words"),
					cmd => {
						warn!("unhandled command: {cmd}");
						Ok(())
					}
				}
			})
			.await?;
		}
		Some(InteractionData::MessageComponent(data)) => {
			handle_interaction_error(app.clone(), interaction, async {
				let subids: Vec<&str> = data.custom_id.split(':').collect();
				info!(?subids, "handle component message");
				match subids.first() {
					Some(&"sprint") => {
						sprint::on_component(app.clone(), interaction, &subids[1..], data)
							.await
							.wrap_err("component: sprint")
					}
					Some(&"debug") => {
						debug::on_component(app.clone(), interaction, &subids[1..], data)
							.await
							.wrap_err("component: debug")
					}
					Some(&"trackbear") => {
						trackbear::on_component(app.clone(), interaction, &subids[1..], data)
							.await
							.wrap_err("component: trackbear")
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
		Some(InteractionData::ModalSubmit(data)) => {
			handle_interaction_error(app.clone(), interaction, async {
				let subids: Vec<&str> = data.custom_id.split(':').collect();
				info!(?subids, "handle modal submit");
				match subids.first() {
					Some(&"sprint") => {
						sprint::on_modal(app.clone(), interaction, &subids[1..], data)
							.await
							.wrap_err("modal: sprint")
					}
					Some(&"trackbear") => {
						trackbear::on_modal(app.clone(), interaction, &subids[1..], data)
							.await
							.wrap_err("modal: trackbear")
					}
					Some(other) => {
						warn!("unhandled modal submit: {other:?}");
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
	app: App,
	interaction: &Interaction,
	task: impl std::future::Future<Output = Result<()>>,
) -> Result<()> {
	if let Err(err) = task.await {
		error!("interaction error: {err:?}");
		app.do_action(CommandError::new(app.clone(), interaction, err).await?)
			.await
	} else {
		Ok(())
	}
}
