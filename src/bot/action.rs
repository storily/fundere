use miette::Result;

pub use self::calc_result::CalcResult;
pub use self::command_ack::CommandAck;
pub use self::command_error::CommandError;
pub use self::sprint_announce::SprintAnnounce;
pub use self::sprint_cancelled::SprintCancelled;
pub use self::sprint_end::SprintEnd;
pub use self::sprint_joined::SprintJoined;
pub use self::sprint_left::SprintLeft;
pub use self::sprint_start::SprintStart;
pub use self::sprint_summary::SprintSummary;
pub use self::sprint_update::SprintUpdate;
pub use self::sprint_warning::SprintWarning;
pub use self::sprint_words_end::SprintWordsEnd;
pub use self::sprint_words_start::SprintWordsStart;

use super::App;

pub mod calc_result;
pub mod command_ack;
pub mod command_error;
pub mod sprint_announce;
pub mod sprint_cancelled;
pub mod sprint_end;
pub mod sprint_joined;
pub mod sprint_left;
pub mod sprint_start;
pub mod sprint_summary;
pub mod sprint_update;
pub mod sprint_warning;
pub mod sprint_words_end;
pub mod sprint_words_start;

#[derive(Debug, Clone)]
pub struct Action {
	pub class: ActionClass,
}

impl Action {
	pub async fn handle(self, app: App) -> Result<()> {
		let args = Args { app };

		use ActionClass::*;
		match self.class {
			CalcResult(data) => data.handle(args).await,
			CommandAck(data) => data.handle(args).await,
			CommandError(data) => data.handle(args).await,
			SprintAnnounce(data) => data.handle(args).await,
			SprintCancelled(data) => data.handle(args).await,
			SprintEnd(data) => data.handle(args).await,
			SprintJoined(data) => data.handle(args).await,
			SprintLeft(data) => data.handle(args).await,
			SprintStart(data) => data.handle(args).await,
			SprintSummary(data) => data.handle(args).await,
			SprintUpdate(data) => data.handle(args).await,
			SprintWarning(data) => data.handle(args).await,
			SprintWordsStart(data) => data.handle(args).await,
			SprintWordsEnd(data) => data.handle(args).await,
		}
	}
}

#[derive(Debug)]
pub struct Args {
	pub app: App,
}

impl From<ActionClass> for Action {
	fn from(class: ActionClass) -> Self {
		Self { class }
	}
}

#[derive(Debug, Clone)]
pub enum ActionClass {
	CalcResult(CalcResult),
	CommandAck(CommandAck),
	CommandError(CommandError),
	SprintAnnounce(SprintAnnounce),
	SprintCancelled(SprintCancelled),
	SprintEnd(SprintEnd),
	SprintJoined(SprintJoined),
	SprintLeft(SprintLeft),
	SprintStart(SprintStart),
	SprintSummary(SprintSummary),
	SprintUpdate(SprintUpdate),
	SprintWarning(SprintWarning),
	SprintWordsStart(SprintWordsStart),
	SprintWordsEnd(SprintWordsEnd),
}
