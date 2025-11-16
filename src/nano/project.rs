use std::fmt::Debug;

use chrono_tz::Tz;
use miette::{miette, Context, IntoDiagnostic, Result};
use nanowrimo::{
	NanoClient, NanoKind, Object, ProjectChallengeObject, ProjectData, ProjectObject, UserObject,
};
use tracing::debug;

use crate::{
	bot::App,
	db::{member::Member, trackbear_login::TrackbearLogin},
};

use super::goal::Goal;

#[derive(Clone, Debug)]
pub struct Project {
	pub id: u64,
	data: ProjectData,
	pub timezone: Tz,
	goals: Vec<Goal>,
}

impl Project {
	pub async fn fetch(app: App, member: Member, id: u64) -> Result<Self> {
		let client = TrackbearLogin::client_for_member(app.clone(), member)
			.await?
			.ok_or_else(|| miette!("No TrackBear login found. Use /trackbear login first."))?;
		Self::fetch_with_client(client, id).await
	}

	pub async fn fetch_with_client(client: NanoClient, id: u64) -> Result<Self> {
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
				Object::ProjectChallenge(ProjectChallengeObject { id, attributes, .. }) => {
					Some(Goal::new(id, timezone, attributes))
				}
				_ => None,
			})
			.collect();
		goals.sort_by_key(|goal| goal.data.starts_at);

		Ok(Self {
			id,
			data: project.data.attributes,
			timezone,
			goals,
		})
	}

	pub fn title(&self) -> &str {
		&self.data.title
	}

	/// Wordcount for current goal
	pub fn wordcount(&self) -> u64 {
		if let Some(goal) = self.current_goal() {
			goal.data.current_count
		} else {
			self.data.unit_count.unwrap_or(0)
		}
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
