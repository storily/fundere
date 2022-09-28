use miette::{IntoDiagnostic, Result};
use tokio::fs;
use twilight_gateway::Intents as TwItents;

#[derive(Debug, Clone, knuffel::Decode)]
pub struct Config {
	#[knuffel(child)]
	pub discord: DiscordConfig,
}

impl Config {
	pub async fn load(path: &str) -> Result<Self> {
		let text = fs::read_to_string(path).await.into_diagnostic()?;
		Ok(knuffel::parse(path, &text)?)
	}
}

#[derive(Debug, Clone, knuffel::Decode)]
pub struct DiscordConfig {
	#[knuffel(property)]
	pub token: String,
	#[knuffel(child)]
	pub intents: IntentsConfig,
}

#[derive(Debug, Clone, knuffel::Decode)]
pub struct IntentsConfig(#[knuffel(children)] pub Vec<Intents>);

impl Default for IntentsConfig {
	fn default() -> Self {
		Self(vec![
			Intents::GuildMessages,
			Intents::GuildVoiceStates,
		])
	}
}

impl IntentsConfig {
	pub fn to_intent(&self) -> TwItents {
		let mut intent = TwItents::empty();
		for i in &self.0 {
			let twi: TwItents = (*i).into();
			intent |= twi;
		}

		intent
	}
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, knuffel::Decode)]
pub enum Intents {
	AutoModerationConfiguration,
	AutoModerationExecution,
	DirectMessages,
	DirectMessageReactions,
	DirectMessageTyping,
	Guilds,
	GuildBans,
	GuildEmojisAndStickers,
	GuildIntegrations,
	GuildInvites,
	GuildMembers,
	GuildMessages,
	GuildMessageReactions,
	GuildMessageTyping,
	GuildPresences,
	GuildScheduledEvents,
	GuildVoiceStates,
	GuildWebhooks,
	MessageContent,
}

impl From<Intents> for TwItents {
	fn from(intent: Intents) -> Self {
		use Intents::*;
		match intent {
			AutoModerationConfiguration => TwItents::AUTO_MODERATION_CONFIGURATION,
			AutoModerationExecution => TwItents::AUTO_MODERATION_EXECUTION,
			DirectMessages => TwItents::DIRECT_MESSAGES,
			DirectMessageReactions => TwItents::DIRECT_MESSAGE_REACTIONS,
			DirectMessageTyping => TwItents::DIRECT_MESSAGE_TYPING,
			Guilds => TwItents::GUILDS,
			GuildBans => TwItents::GUILD_BANS,
			GuildEmojisAndStickers => TwItents::GUILD_EMOJIS_AND_STICKERS,
			GuildIntegrations => TwItents::GUILD_INTEGRATIONS,
			GuildInvites => TwItents::GUILD_INVITES,
			GuildMembers => TwItents::GUILD_MEMBERS,
			GuildMessages => TwItents::GUILD_MESSAGES,
			GuildMessageReactions => TwItents::GUILD_MESSAGE_REACTIONS,
			GuildMessageTyping => TwItents::GUILD_MESSAGE_TYPING,
			GuildPresences => TwItents::GUILD_PRESENCES,
			GuildScheduledEvents => TwItents::GUILD_SCHEDULED_EVENTS,
			GuildVoiceStates => TwItents::GUILD_VOICE_STATES,
			GuildWebhooks => TwItents::GUILD_WEBHOOKS,
			MessageContent => TwItents::MESSAGE_CONTENT,
		}
	}
}
