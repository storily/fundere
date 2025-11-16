#![allow(deprecated)] // chrono's .date()

use std::{str::FromStr, time::Duration as StdDuration};

use chrono::{Duration, Utc};
use futures_util::future::try_join_all;
use miette::{miette, Context, IntoDiagnostic, Result};
use tracing::{debug, error, info, warn};
use twilight_model::application::{
	command::{Command, CommandType},
	interaction::{
		application_command::{CommandData, CommandDataOption, CommandOptionValue},
		message_component::MessageComponentInteractionData,
		modal::ModalInteractionData,
		Interaction,
	},
};
use twilight_util::builder::command::{
	CommandBuilder, IntegerBuilder, StringBuilder, SubCommandBuilder,
};
use uuid::Uuid;

use crate::{
	bot::{
		action::{
			CommandAck, ComponentAck, SprintAnnounce, SprintCancelled, SprintEnd, SprintEndWarning,
			SprintJoined, SprintLeft, SprintSaveWords, SprintStart, SprintStartWarning,
			SprintSummary, SprintUpdate, SprintWordsEnd, SprintWordsStart,
		},
		context::{GenericResponse, GenericResponseData, Timer},
		utils::{
			command::{get_integer, get_string},
			time::parse_when_relative_to,
		},
		words::{save_words as save_words_action, SaveWords},
		App,
	},
	db::{
		channel::Channel,
		member::Member,
		project::Project,
		sprint::{Sprint, SprintStatus},
		trackbear_login::TrackbearLogin,
	},
	error_ext::ErrorExt,
};

#[tracing::instrument]
pub fn command() -> Result<Command> {
	CommandBuilder::new(
		"sprint",
		"Experimental new-gen wordwar/sprint command",
		CommandType::ChatInput,
	)
	.option(
		SubCommandBuilder::new("new", "Schedule a new sprint")
			.option(
				StringBuilder::new(
					"when",
					"When to start the sprint, either in clock time (08:30), or in relative time (15m)",
				)
				.required(true)
			)
			.option(
				IntegerBuilder::new(
					"duration",
					"Duration of the sprint in minutes (defaults to 20)",
				)
			)
	)
	.option(SubCommandBuilder::new("list", "List all current sprints"))
	.option(
		SubCommandBuilder::new("summary", "Show the summary of a sprint")
			.option(
				IntegerBuilder::new(
					"sprint",
					"Short sprint ID, like 3349",
				)
				.required(true)
			)
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
		Some(("new", opts)) => sprint_new(app.clone(), interaction, opts)
			.await
			.wrap_err("command: new")?,
		Some(("list", opts)) => sprint_list(app.clone(), interaction, opts)
			.await
			.wrap_err("command: list")?,
		Some(("summary", opts)) => sprint_summary(app.clone(), interaction, opts)
			.await
			.wrap_err("command: summary")?,
		Some((other, _)) => warn!("unhandled sprint subcommand: {other}"),
		_ => error!("unreachable bare sprint command"),
	}

	Ok(())
}

pub async fn on_component(
	app: App,
	interaction: &Interaction,
	subids: &[&str],
	component_data: &MessageComponentInteractionData,
) -> Result<()> {
	debug!(?subids, ?component_data, "sprint component action");

	match subids {
		["join", uuid] => sprint_join(app.clone(), interaction, uuid)
			.await
			.wrap_err("action: join")?,
		["leave", uuid] => sprint_leave(app.clone(), interaction, uuid)
			.await
			.wrap_err("action: leave")?,
		["cancel", uuid] => sprint_cancel(app.clone(), interaction, uuid)
			.await
			.wrap_err("action: cancel")?,
		["start-words", uuid] => sprint_words_start(app.clone(), interaction, uuid)
			.await
			.wrap_err("action: words modal: start")?,
		["end-words", uuid] => sprint_words_end(app.clone(), interaction, uuid)
			.await
			.wrap_err("action: words modal: end")?,
		["save-words", sprint_id, project_id] => {
			save_words(app.clone(), interaction, sprint_id, project_id)
				.await
				.wrap_err("action: save words")?
		}
		["save-never", nano_login_id] => save_never(app.clone(), interaction, nano_login_id)
			.await
			.wrap_err("action: save words: don't ask again")?,
		id => warn!(?id, "unhandled sprint component action"),
	}

	Ok(())
}

pub async fn on_modal(
	app: App,
	interaction: &Interaction,
	subids: &[&str],
	component_data: &ModalInteractionData,
) -> Result<()> {
	debug!(?subids, ?component_data, "sprint modal action");

	match subids {
		["set-words", "start", uuid] => sprint_set_words(
			app.clone(),
			interaction,
			uuid,
			component_data,
			"words_start",
		)
		.await
		.wrap_err("action: words modal: starting")?,
		["set-words", "end", uuid] => {
			sprint_set_words(app.clone(), interaction, uuid, component_data, "words_end")
				.await
				.wrap_err("action: words modal: ending")?
		}
		id => warn!(?id, "unhandled sprint modal action"),
	}

	Ok(())
}

pub async fn load_from_db(app: App) -> Result<()> {
	let finished = Sprint::get_all_finished_but_not_summaried(app.clone()).await?;
	let mut need_summarying = 0;
	for sprint in finished {
		if sprint
			.all_participants_have_ending_words(app.clone())
			.await?
		{
			need_summarying += 1;
			app.do_action(SprintSummary::new_from_db(app.clone(), sprint).await?)
				.await?;
		}
	}

	let ended_but_we_are_late = Sprint::get_all_finished_but_not_ended(app.clone()).await?;
	let mut ended_late = 0;
	for sprint in ended_but_we_are_late {
		ended_late += 1;
		app.do_action(SprintEnd::new(&sprint)).await?;
	}

	let current = Sprint::get_all_current(app.clone()).await?;

	let mut rescheduled = 0;
	let mut actioned_late = 0;
	for sprint in current {
		match sprint.status {
			SprintStatus::Initial => {
				actioned_late += 1;
				app.do_action(SprintAnnounce::new_from_db(app.clone(), sprint).await?)
					.await?;
			}
			SprintStatus::Announced => {
				// UNWRAP: warning_in uses saturating_sub, will never be negative
				let warning_in = sprint.warning_in().to_std().unwrap();
				if warning_in.is_zero() {
					let starting_in = sprint.starting_in();
					if starting_in > Duration::seconds(2) {
						actioned_late += 1;
						app.do_action(SprintStartWarning::new(&sprint)).await?;
					} else if let Ok(starting_in) = starting_in.to_std() {
						// implicitly checks that starting_in >= zero
						rescheduled += 1;
						app.send_timer(Timer::new_after(starting_in, SprintStart::new(&sprint))?)
							.await?;
					} else {
						actioned_late += 1;
						app.do_action(SprintStart::new(&sprint)).await?;
					}
				} else {
					rescheduled += 1;
					app.send_timer(Timer::new_after(
						warning_in,
						SprintStartWarning::new(&sprint),
					)?)
					.await?;
				}
			}
			SprintStatus::Started => {
				if let Ok(ending_in) = sprint.ending_in().to_std() {
					// implicitly checks that ending_in >= zero
					rescheduled += 1;
					app.send_timer(Timer::new_after(ending_in, SprintEnd::new(&sprint))?)
						.await?;
					app.send_timer(Timer::new_after(
						ending_in.saturating_sub(std::time::Duration::from_secs(30)),
						SprintEndWarning::new(&sprint),
					)?)
					.await?;
				} else {
					warn!("sprint in init loaded from sprints_current that is started but is beyond end");
					actioned_late += 1;
					app.do_action(SprintEnd::new(&sprint)).await?;
				}
			}
			_ => warn!(?sprint, "unhandled case of sprint loaded from db"),
		}
	}

	info!(%actioned_late, %rescheduled, %ended_late, %need_summarying, "loaded sprints from db");

	Ok(())
}

async fn sprint_new(
	app: App,
	interaction: &Interaction,
	options: &[CommandDataOption],
) -> Result<()> {
	let duration = get_integer(options, "duration").unwrap_or(20);
	if duration <= 0 {
		return Err(miette!("duration must be positive"));
	}
	let duration = Duration::minutes(duration);

	let channel = Channel::try_from(interaction)?;
	let member = Member::try_from(interaction)?;
	app.do_action(CommandAck::new(interaction)).await.log().ok();

	// TrackBear doesn't provide timezone info, so we use UTC for sprint scheduling
	// TODO: Consider storing user timezone preferences in the database
	let now = Utc::now().with_timezone(&chrono_tz::UTC);

	let when = parse_when_relative_to(now.time(), get_string(options, "when").unwrap_or("15m"))?;

	let now_with_time = now.date().and_time(when).ok_or(miette!("invalid time"))?;
	let starting = if now_with_time < now {
		(now + Duration::days(1))
			.date()
			.and_time(when)
			.ok_or(miette!("invalid time"))?
	} else {
		now_with_time
	};

	debug!(%starting, %duration, ?channel, ?member, "recording sprint");
	let sprint =
		Sprint::create(app.clone(), starting, duration, &interaction.token, member).await?;

	app.do_action(
		SprintAnnounce::new(app.clone(), interaction, sprint)
			.await
			.wrap_err("rendering announce")?,
	)
	.await?;

	Ok(())
}

async fn sprint_join(app: App, interaction: &Interaction, uuid: &str) -> Result<()> {
	let uuid = Uuid::from_str(uuid).into_diagnostic()?;
	let member = Member::try_from(interaction)?;
	let sprint = Sprint::get_current(app.clone(), uuid)
		.await
		.wrap_err("that sprint isn't current")?;

	if sprint.status >= SprintStatus::Ended {
		return Err(miette!("sprint has already ended"));
	}

	app.do_action(ComponentAck::ephemeral(interaction))
		.await
		.log()
		.ok();

	sprint.join(app.clone(), member).await?;

	app.do_action(SprintJoined::new(interaction, &sprint)?)
		.await?;

	app.do_action(SprintUpdate::new(&sprint)).await?;

	Ok(())
}

async fn sprint_leave(app: App, interaction: &Interaction, uuid: &str) -> Result<()> {
	let uuid = Uuid::from_str(uuid).into_diagnostic()?;

	let member = Member::try_from(interaction)?;
	let sprint = Sprint::get_current(app.clone(), uuid)
		.await
		.wrap_err("that sprint isn't current")?;

	if sprint.status >= SprintStatus::Ended {
		return Err(miette!("sprint has already ended"));
	}

	app.do_action(ComponentAck::new(interaction))
		.await
		.log()
		.ok();

	sprint.leave(app.clone(), member).await?;

	app.do_action(SprintLeft::new(interaction, &sprint)?)
		.await?;

	if sprint.participants(app.clone()).await?.is_empty() {
		let user = interaction
			.member
			.as_ref()
			.and_then(|m| m.user.as_ref())
			.ok_or(miette!("can only leave sprint from a guild"))?;

		sprint.cancel(app.clone()).await?;
		app.do_action(SprintCancelled::new(interaction, &sprint, user)?)
			.await?;
	} else {
		app.do_action(SprintUpdate::new(&sprint)).await?;
	}

	Ok(())
}

async fn sprint_cancel(app: App, interaction: &Interaction, uuid: &str) -> Result<()> {
	let uuid = Uuid::from_str(uuid).into_diagnostic()?;
	let sprint = Sprint::get_current(app.clone(), uuid)
		.await
		.wrap_err("that sprint isn't current")?;

	let user = interaction
		.member
		.as_ref()
		.and_then(|m| m.user.as_ref())
		.ok_or(miette!("can only cancel sprint from a guild"))?;

	if sprint.status >= SprintStatus::Ended {
		return Err(miette!("sprint has already ended"));
	}

	app.do_action(ComponentAck::new(interaction))
		.await
		.log()
		.ok();

	sprint.cancel(app.clone()).await?;

	app.do_action(SprintCancelled::new(interaction, &sprint, user)?)
		.await?;

	Ok(())
}

async fn sprint_words_start(app: App, interaction: &Interaction, uuid: &str) -> Result<()> {
	let uuid = Uuid::from_str(uuid).into_diagnostic()?;
	let member = Member::try_from(interaction)?;
	let sprint = Sprint::get(app.clone(), uuid)
		.await
		.wrap_err("sprint not found")?;

	if sprint.is_cancelled() {
		return Err(miette!("sprint was cancelled"));
	}

	app.do_action(SprintWordsStart::new(interaction, sprint.id, member))
		.await?;

	Ok(())
}

async fn sprint_words_end(app: App, interaction: &Interaction, uuid: &str) -> Result<()> {
	let uuid = Uuid::from_str(uuid).into_diagnostic()?;
	let member = Member::try_from(interaction)?;
	let sprint = Sprint::get(app.clone(), uuid)
		.await
		.wrap_err("sprint not found")?;

	if sprint.is_cancelled() {
		return Err(miette!("sprint was cancelled"));
	}

	app.do_action(SprintWordsEnd::new(interaction, sprint.id, member))
		.await?;

	Ok(())
}

async fn sprint_set_words(
	app: App,
	interaction: &Interaction,
	uuid: &str,
	data: &ModalInteractionData,
	column: &str,
) -> Result<()> {
	let uuid = Uuid::from_str(uuid).into_diagnostic()?;
	let member = Member::try_from(interaction)?;
	let sprint = Sprint::get(app.clone(), uuid)
		.await
		.wrap_err("sprint not found")?;

	if sprint.is_cancelled() {
		return Err(miette!("sprint was cancelled"));
	}

	let words = data
		.components
		.iter()
		.flat_map(|row| row.components.iter())
		.find_map(|component| {
			if component.custom_id == "words" {
				Some(&component.value)
			} else {
				None
			}
		})
		.ok_or_else(|| miette!("words is a required field"))?
		.as_deref()
		.map(|words| i32::from_str(words).into_diagnostic())
		.transpose()?
		.unwrap_or(0);

	app.do_action(ComponentAck::ephemeral(interaction))
		.await
		.log()
		.ok();

	sprint.set_words(app.clone(), member, words, column).await?;

	if column == "words_end" {
		if let Some(act) = SprintSaveWords::new(app.clone(), interaction, &sprint, member).await? {
			app.do_action(act).await?;
		}

		if sprint
			.all_participants_have_ending_words(app.clone())
			.await?
		{
			// Delay so that it hopefully doesn't inherit the ephemeralness
			app.send_timer(Timer::new_after(
				StdDuration::from_secs(1),
				SprintSummary::new(app.clone(), interaction, sprint).await?,
			)?)
			.await?;
		}
	}

	Ok(())
}

async fn sprint_list(
	app: App,
	interaction: &Interaction,
	_options: &[CommandDataOption],
) -> Result<()> {
	let sprints = Sprint::get_all_current(app.clone()).await?;
	app.do_action(CommandAck::new(interaction)).await.log().ok();

	let content = if sprints.is_empty() {
		"No sprints are currently running.".to_string()
	} else {
		try_join_all(sprints.into_iter().map(|sprint| {
			let app = app.clone();
			async move { sprint.status_text(app, false).await }
		}))
		.await?
		.join("\n")
	};

	app.send_response(GenericResponse::from_interaction(
		interaction,
		GenericResponseData {
			content: Some(content),
			..Default::default()
		},
	))
	.await?;

	Ok(())
}

async fn sprint_summary(
	app: App,
	interaction: &Interaction,
	options: &[CommandDataOption],
) -> Result<()> {
	let shortid =
		get_integer(options, "sprint").ok_or_else(|| miette!("sprint is a required field"))?;
	debug!(?shortid, "got shortid");
	app.do_action(CommandAck::new(interaction)).await.log().ok();

	let sprint = Sprint::get_from_shortid(
		app.clone(),
		shortid
			.try_into()
			.into_diagnostic()
			.wrap_err("sprint ID is too large")?,
	)
	.await
	.wrap_err("sprint not found")?;

	app.do_action(SprintSummary::new(app.clone(), interaction, sprint).await?)
		.await
}

async fn save_words(
	app: App,
	interaction: &Interaction,
	sprint_id: &str,
	project_id: &str,
) -> Result<()> {
	let member = Member::try_from(interaction)?;
	let sprint_id = Uuid::from_str(sprint_id).into_diagnostic()?;
	let project_id = Uuid::from_str(project_id).into_diagnostic()?;
	app.do_action(ComponentAck::new(interaction))
		.await
		.log()
		.ok();

	let sprint = Sprint::get(app.clone(), sprint_id)
		.await
		.wrap_err("sprint not found")?;
	let participant = sprint.participant(app.clone(), member).await?;
	let words = SaveWords::Relative(participant.words_written().unwrap_or(0).into());

	let project = Project::get(app.clone(), project_id)
		.await
		.and_then(|opt| opt.ok_or_else(|| miette!("no project for {:?}", member)))
		.wrap_err("project not found")?;

	let client = TrackbearLogin::client_for_member(app.clone(), member)
		.await?
		.ok_or_else(|| miette!("no trackbear login for {:?}", member))?;

	save_words_action(app, interaction, &client, &project, words).await
}

async fn save_never(app: App, interaction: &Interaction, login_id: &str) -> Result<()> {
	let login_id = Uuid::from_str(login_id).into_diagnostic()?;
	app.do_action(ComponentAck::ephemeral(interaction))
		.await
		.log()
		.ok();

	let Some(mut login) = TrackbearLogin::get(app.clone(), login_id)
		.await
		.wrap_err("login not found")?
	else {
		return Ok(());
	};

	login.ask_me(app.clone(), false).await?;

	app.send_response(GenericResponse::from_interaction(
		interaction,
		GenericResponseData {
			ephemeral: true,
			content: Some("Ok, no worries.".into()),
			..Default::default()
		},
	))
	.await
	.map(drop)
}
