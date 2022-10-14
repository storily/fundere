use std::{path::Path, str::FromStr, time::Duration};

use miette::{IntoDiagnostic, Result};
use tokio::fs;
use tokio_postgres::{
	config::{ChannelBinding, Config as PgConfig, SslMode, TargetSessionAttrs},
	tls::NoTlsStream,
	Client, Connection, NoTls, Socket,
};
use twilight_gateway::Intents as TwItents;

#[derive(Debug, Clone, knuffel::Decode)]
pub struct Config {
	#[knuffel(child)]
	pub discord: DiscordConfig,

	#[knuffel(child, default)]
	pub db: DbConfig,

	#[knuffel(child, default)]
	pub internal: InternalConfig,
}

impl Config {
	pub async fn load(path: impl AsRef<Path>) -> Result<Self> {
		let path = path.as_ref();
		let text = fs::read_to_string(path).await.into_diagnostic()?;
		Ok(knuffel::parse(path.to_string_lossy().as_ref(), &text)?)
	}
}

#[derive(Debug, Clone, knuffel::Decode)]
pub struct DiscordConfig {
	#[knuffel(property)]
	pub token: String,
	#[knuffel(property)]
	pub app_id: u64,
	#[knuffel(child)]
	pub intents: IntentsConfig,
}

#[derive(Debug, Clone, knuffel::Decode)]
pub struct IntentsConfig(#[knuffel(children)] pub Vec<Intents>);

impl Default for IntentsConfig {
	fn default() -> Self {
		Self(vec![Intents::GuildMessages, Intents::GuildVoiceStates])
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

#[derive(Debug, Clone, Default, knuffel::Decode)]
pub struct DbConfig {
	#[knuffel(child, unwrap(argument))]
	pub url: Option<String>,

	#[knuffel(child, unwrap(argument))]
	pub user: Option<String>,

	#[knuffel(child, unwrap(argument))]
	pub password: Option<String>,

	#[knuffel(child, unwrap(argument))]
	pub options: Option<String>,

	#[knuffel(child, unwrap(argument))]
	pub application_name: Option<String>,

	#[knuffel(child, default)]
	pub ssl_mode: ConfigSslMode,

	#[knuffel(children(name = "host"), unwrap(argument))]
	pub hosts: Vec<String>,

	#[knuffel(children(name = "port"), unwrap(argument))]
	pub ports: Vec<u16>,

	#[knuffel(child, unwrap(argument))]
	pub connect_timeout: Option<u16>,

	#[knuffel(child, unwrap(argument))]
	pub keepalives: Option<bool>,

	#[knuffel(child, unwrap(argument))]
	pub keepalives_idle: Option<u64>,

	#[knuffel(child, default)]
	pub target_session_attrs: ConfigTargetSessionAttrs,

	#[knuffel(child, default)]
	pub channel_binding: ConfigChannelBinding,
}

impl DbConfig {
	pub async fn connect(&self) -> Result<(Client, Connection<Socket, NoTlsStream>)> {
		let mut config = if let Some(url) = &self.url {
			PgConfig::from_str(&url).into_diagnostic()?
		} else {
			PgConfig::new()
		};

		for host in &self.hosts {
			config.host(host);
		}

		for port in &self.ports {
			config.port(*port);
		}

		if let Some(v) = &self.user {
			config.user(v);
		}
		if let Some(v) = &self.password {
			config.password(v);
		}
		if let Some(v) = &self.options {
			config.options(v);
		}
		if let Some(v) = &self.application_name {
			config.application_name(v);
		}

		if let Some(v) = self.keepalives {
			config.keepalives(v);
		}
		if let Some(v) = self.keepalives_idle {
			config.keepalives_idle(Duration::from_secs(v));
		}
		if let Some(v) = self.connect_timeout {
			config.connect_timeout(Duration::from_secs(v as _));
		}

		config.ssl_mode(self.ssl_mode.into());
		config.target_session_attrs(self.target_session_attrs.into());
		config.channel_binding(self.channel_binding.into());

		config.connect(NoTls).await.into_diagnostic()
	}
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, knuffel::Decode)]
#[non_exhaustive]
pub enum ConfigSslMode {
	Disable,
	#[default]
	Prefer,
	Require,
}

impl From<ConfigSslMode> for SslMode {
	fn from(c: ConfigSslMode) -> Self {
		match c {
			ConfigSslMode::Disable => Self::Disable,
			ConfigSslMode::Prefer => Self::Prefer,
			ConfigSslMode::Require => Self::Require,
		}
	}
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, knuffel::Decode)]
#[non_exhaustive]
pub enum ConfigTargetSessionAttrs {
	#[default]
	Any,
	ReadWrite,
}

impl From<ConfigTargetSessionAttrs> for TargetSessionAttrs {
	fn from(c: ConfigTargetSessionAttrs) -> Self {
		match c {
			ConfigTargetSessionAttrs::Any => Self::Any,
			ConfigTargetSessionAttrs::ReadWrite => Self::ReadWrite,
		}
	}
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, knuffel::Decode)]
#[non_exhaustive]
pub enum ConfigChannelBinding {
	Disable,
	#[default]
	Prefer,
	Require,
}

impl From<ConfigChannelBinding> for ChannelBinding {
	fn from(c: ConfigChannelBinding) -> Self {
		match c {
			ConfigChannelBinding::Disable => Self::Disable,
			ConfigChannelBinding::Prefer => Self::Prefer,
			ConfigChannelBinding::Require => Self::Require,
		}
	}
}

#[derive(Debug, Clone, knuffel::Decode)]
pub struct InternalConfig {
	#[knuffel(child, unwrap(argument), default = Self::default().timer_buffer)]
	pub timer_buffer: usize,
}

impl Default for InternalConfig {
	fn default() -> Self {
		Self { timer_buffer: 16 }
	}
}
