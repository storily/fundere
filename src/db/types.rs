use miette::{IntoDiagnostic, Result};
use postgres_types::{FromSql, ToSql};
use twilight_mention::{fmt::MentionFormat, Mention};
use twilight_model::{
	guild::Member as DiscordMember,
	id::{marker::UserMarker, Id},
	user::User,
};

use crate::bot::App;

#[derive(Debug, Clone, ToSql, FromSql)]
#[postgres(name = "member_t")]
struct MemberInner {
	pub guild_id: i64,
	pub user_id: i64,
}

#[derive(Debug, Clone, ToSql, FromSql)]
pub struct Member(MemberInner);

impl Member {
	pub async fn to_user(&self, app: App) -> Result<User> {
		app.client
			.user(Id::new(self.0.user_id as _))
			.exec()
			.await
			.into_diagnostic()?
			.model()
			.await
			.into_diagnostic()
	}

	pub async fn to_member(&self, app: App) -> Result<DiscordMember> {
		app.client
			.guild_member(Id::new(self.0.guild_id as _), Id::new(self.0.user_id as _))
			.exec()
			.await
			.into_diagnostic()?
			.model()
			.await
			.into_diagnostic()
	}
}

impl Mention<Id<UserMarker>> for Member {
	fn mention(&self) -> MentionFormat<Id<UserMarker>> {
		Id::new(self.0.user_id as _).mention()
	}
}
