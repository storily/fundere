use std::{ffi::OsString, path::PathBuf};

use clap::{Parser, Subcommand};
use miette::{IntoDiagnostic, Result};

pub(crate) mod bot;
pub(crate) mod config;
pub(crate) mod db;

#[tokio::main]
async fn main() -> Result<()> {
	tracing_subscriber::fmt::init();

	let cli = Cli::parse();

	let config = config::Config::load(&cli.config).await?;

	match cli.command {
		Command::Sqlx { args } => sqlx(config, args).await,
		Command::Start => bot::start(config).await,
	}
}

/// Sassbot (Fundere edition)
#[derive(Clone, Debug, Parser)]
#[command(author, version, about, long_about = None)]
struct Cli {
	/// Location of the config file
	#[arg(short, long, value_name = "FILE", default_value = "config.kdl")]
	config: PathBuf,

	#[command(subcommand)]
	command: Command,
}

#[derive(Clone, Debug, Subcommand)]
enum Command {
	/// Run sqlx with the DATABASE_URL env set
	Sqlx {
		#[arg(raw = true)]
		args: Vec<OsString>,
	},

	/// Start bot
	Start,
}

async fn sqlx(config: config::Config, args: Vec<OsString>) -> Result<()> {
	let mut proc = tokio::process::Command::new("sqlx")
		.args(args)
		.env("DATABASE_URL", config.db.url)
		.spawn()
		.into_diagnostic()?;
	proc.wait().await.into_diagnostic()?;
	Ok(())
}
