macro_rules! action {
	(
		$($modname:ident : $typename:ident),*
	) => {
		$(
			pub mod $modname;
			pub use self::$modname::$typename;
		)*

		#[derive(Debug, Clone)]
		pub struct Action {
			pub class: ActionClass,
		}

		#[derive(Debug)]
		pub struct Args {
			pub app: super::App,
		}

		impl Action {
			pub async fn handle(self, app: super::App) -> ::miette::Result<()> {
				let args = Args { app };

				use ActionClass::*;
				match self.class {
					$($typename(action) => action.handle(args).await),*
				}
			}
		}

		#[derive(Debug, Clone)]
		pub enum ActionClass {
			$($typename($typename)),*
		}

		impl From<ActionClass> for Action {
			fn from(class: ActionClass) -> Self {
				Self { class }
			}
		}
	};
}

action!(
	calc_result: CalcResult,
	command_ack: CommandAck,
	component_ack: ComponentAck,
	command_error: CommandError,
	nanowrimo_login_confirm: NanowrimoLoginConfirm,
	nanowrimo_login_modal: NanowrimoLoginModal,
	sprint_announce: SprintAnnounce,
	sprint_cancelled: SprintCancelled,
	sprint_end: SprintEnd,
	sprint_joined: SprintJoined,
	sprint_left: SprintLeft,
	sprint_start: SprintStart,
	sprint_summary: SprintSummary,
	sprint_update: SprintUpdate,
	sprint_end_warning: SprintEndWarning,
	sprint_start_warning: SprintStartWarning,
	sprint_words_end: SprintWordsEnd,
	sprint_words_start: SprintWordsStart
);
