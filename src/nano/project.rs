use std::fmt::Debug;

use chrono::{DateTime, TimeZone, Utc};
use chrono_tz::Tz;
use miette::{miette, Context, IntoDiagnostic, Result};
use nanowrimo::{
	NanoKind, Object, ProjectChallengeData, ProjectChallengeObject, ProjectData, ProjectObject,
	UserObject,
};
use tokio_postgres::Row;
use tracing::debug;
use uuid::Uuid;

use crate::{
	bot::App,
	db::{member::Member, nanowrimo_login::NanowrimoLogin},
};

use super::goal::Goal;

#[derive(Debug)]
pub struct Project {
	app: App,
	data: ProjectData,
	timezone: Tz,
	goals: Vec<Goal>,
}

impl Project {
	pub async fn fetch(app: App, member: Member, id: u64) -> Result<Self> {
		let client = NanowrimoLogin::client_for_member_or_default(app.clone(), member).await?;

		let project = client
			.get_id_include::<ProjectObject>(NanoKind::Project, id, &[NanoKind::ProjectChallenge])
			.await
			.into_diagnostic()
			.wrap_err("nano: fetch project and challenges")?;
		debug!(?id, ?project, "fetched project from nano");

		let user = client
			.get_id::<UserObject>(NanoKind::User, project.data.attributes.user_id)
			.await
			.into_diagnostic()
			.wrap_err("nano: fetch project user")?;
		let timezone: Tz = user
			.data
			.attributes
			.time_zone
			.parse()
			.map_err(|err| miette!("nano: fetch project user timezone: {}", err))?;

		let mut goals: Vec<Goal> = project
			.included
			.unwrap_or_default()
			.into_iter()
			.filter_map(|obj| match obj {
				Object::ProjectChallenge(ProjectChallengeObject { attributes, .. }) => {
					Some(Goal::new(app.clone(), timezone.clone(), attributes))
				}
				_ => None,
			})
			.collect();
		goals.sort_by_key(|goal| goal.data.starts_at);

		Ok(Self {
			app,
			data: project.data.attributes,
			timezone,
			goals,
		})
	}

	pub fn title(&self) -> &str {
		&self.data.title
	}

	/// Words that are already accounted for in past goals.
	fn accounted_words(&self) -> u64 {
		if self.goals.is_empty() {
			return 0;
		}

		let mut past_goals: Vec<&Goal> = self
			.goals
			.iter()
			.filter(|goal| !goal.is_current())
			.collect();
		if past_goals.len() == self.goals.len() {
			past_goals.sort_by_key(|goal| goal.data.starts_at);
			past_goals.pop();
		}

		past_goals
			.into_iter()
			.map(|goal| goal.data.current_count)
			.sum()
	}

	/// Wordcount for current goal
	pub fn wordcount(&self) -> u64 {
		self.data
			.unit_count
			.unwrap_or(0)
			.saturating_sub(self.accounted_words())
	}

	fn current_goals(&self) -> Vec<&Goal> {
		if self.goals.is_empty() {
			return Vec::new();
		}

		let mut active: Vec<&Goal> = self.goals.iter().filter(|goal| goal.is_current()).collect();
		if !active.is_empty() {
			active.sort_by_key(|goal| goal.data.starts_at);
			return active;
		}

		// fallback on most recent goal
		self.goals.last().map_or(Vec::new(), |goal| vec![goal])
	}

	pub fn current_goal(&self) -> Option<&Goal> {
		self.current_goals().last().copied()
	}
}
