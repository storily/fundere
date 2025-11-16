use chrono::NaiveDate;
use miette::{miette, Result};

use super::client::{
	CreateTallyRequest, Goal, Measure, Project as TbProject, Tally, TrackbearClient,
};

/// A TrackBear project with associated goals and tallies
#[derive(Debug, Clone)]
pub struct Project {
	pub project: TbProject,
	pub goals: Vec<Goal>,
}

impl Project {
	/// Fetch a project by ID with its associated goals
	pub async fn fetch(client: &TrackbearClient, project_id: i64) -> Result<Self> {
		let projects = client.list_projects().await?;
		let project = projects
			.into_iter()
			.find(|p| p.id == project_id)
			.ok_or_else(|| miette!("Project with ID {} not found", project_id))?;

		let all_goals = client.list_goals().await?;
		let goals = all_goals
			.into_iter()
			.filter(|g| g.work_ids.contains(&project_id))
			.collect();

		Ok(Self { project, goals })
	}

	/// Get the project title
	pub fn title(&self) -> &str {
		&self.project.title
	}

	/// Get the current word count (including starting balance)
	pub fn word_count(&self) -> i64 {
		self.project.totals.word.unwrap_or_default()
	}

	/// Find the currently active goal for this project
	pub fn current_goal(&self) -> Option<&Goal> {
		let today = chrono::Utc::now().date_naive();

		// Find goals that are currently active (between start and end date)
		let mut active_goals: Vec<&Goal> = self
			.goals
			.iter()
			.filter(|g| {
				if let (Some(start), Some(end)) = (
					g.start_date
						.as_ref()
						.and_then(|d| NaiveDate::parse_from_str(d, "%Y-%m-%d").ok()),
					g.end_date
						.as_ref()
						.and_then(|d| NaiveDate::parse_from_str(d, "%Y-%m-%d").ok()),
				) {
					today >= start && today <= end
				} else {
					false
				}
			})
			.collect();

		// Sort by end date (prefer goals ending sooner)
		active_goals.sort_by(|a, b| a.end_date.cmp(&b.end_date));

		active_goals.first().copied()
	}

	/// Get all currently active goals for this project
	pub fn current_goals(&self) -> Vec<&Goal> {
		let today = chrono::Utc::now().date_naive();

		self.goals
			.iter()
			.filter(|g| {
				if let (Some(start), Some(end)) = (
					g.start_date
						.as_ref()
						.and_then(|d| NaiveDate::parse_from_str(d, "%Y-%m-%d").ok()),
					g.end_date
						.as_ref()
						.and_then(|d| NaiveDate::parse_from_str(d, "%Y-%m-%d").ok()),
				) {
					today >= start && today <= end
				} else {
					false
				}
			})
			.collect()
	}

	/// Calculate progress for a given goal
	pub fn goal_progress(&self, goal: &Goal) -> Option<GoalProgress> {
		// Only calculate for word-based goals
		if goal.parameters.threshold.measure != Measure::Word {
			return None;
		}

		let current = self.word_count();
		let target = goal.parameters.threshold.count;

		// Parse dates
		let today = chrono::Utc::now().date_naive();
		let (Some(start_date), Some(end_date)) = (
			goal.start_date
				.as_ref()
				.and_then(|d| NaiveDate::parse_from_str(d, "%Y-%m-%d").ok()),
			goal.end_date
				.as_ref()
				.and_then(|d| NaiveDate::parse_from_str(d, "%Y-%m-%d").ok()),
		) else {
			return None;
		};

		// Calculate total days and days elapsed
		let total_days = (end_date - start_date).num_days() + 1;
		let days_elapsed = (today - start_date).num_days().max(0) + 1;
		let days_remaining = (end_date - today).num_days().max(0);

		// Calculate expected progress
		let expected_today = if total_days > 0 {
			(target as f64 * days_elapsed as f64 / total_days as f64) as i64
		} else {
			target
		};

		// Calculate daily target
		let daily_target = if total_days > 0 {
			target / total_days
		} else {
			0
		};

		// Words ahead/behind schedule
		let diff = current - expected_today;

		// Calculate words needed per day to finish on time
		let words_per_day_to_finish = if days_remaining > 0 {
			(target - current).max(0) / days_remaining
		} else {
			0
		};

		let percent = if target > 0 {
			current as f64 / target as f64 * 100.0
		} else {
			0.0
		};

		Some(GoalProgress {
			current,
			target,
			percent,
			days_elapsed: days_elapsed as i64,
			days_remaining: days_remaining as i64,
			daily_target,
			words_ahead_behind: diff,
			words_per_day_to_finish,
			achieved: goal.achieved || current >= target,
		})
	}

	/// Create a tally for this project
	pub async fn add_tally(
		&self,
		client: &TrackbearClient,
		count: i64,
		set_total: bool,
		note: Option<String>,
	) -> Result<Tally> {
		let date = chrono::Utc::now().date_naive();

		let request = CreateTallyRequest {
			date: date.format("%Y-%m-%d").to_string(),
			measure: Measure::Word,
			count,
			note: note.unwrap_or_default(),
			work_id: self.project.id,
			set_total,
			tags: vec![],
		};

		client.create_tally(request).await
	}
}

#[derive(Debug, Clone)]
pub struct GoalProgress {
	pub current: i64,
	pub target: i64,
	pub percent: f64,
	pub days_elapsed: i64,
	pub days_remaining: i64,
	pub daily_target: i64,
	pub words_ahead_behind: i64,
	pub words_per_day_to_finish: i64,
	pub achieved: bool,
}

impl GoalProgress {
	/// Format the progress as a human-readable string
	pub fn format_tracking(&self) -> String {
		if self.words_ahead_behind == 0 {
			"on track".to_string()
		} else if self.words_ahead_behind > 0 {
			format!("{} ahead", format_count(self.words_ahead_behind))
		} else {
			format!("{} behind", format_count(-self.words_ahead_behind))
		}
	}
}

/// Format a count with k/M suffix
fn format_count(n: i64) -> String {
	if n < 1000 {
		n.to_string()
	} else if n < 10_000 {
		format!("{:.1}k", (n as f64) / 1_000.0)
	} else if n < 1_000_000 {
		format!("{:.0}k", (n as f64) / 1_000.0)
	} else if n < 10_000_000 {
		format!("{:.1}M", (n as f64) / 1_000_000.0)
	} else {
		format!("{:.0}M", (n as f64) / 1_000_000.0)
	}
}
