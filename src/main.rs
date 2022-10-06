use std::path::PathBuf;

use clap::{Parser, Subcommand};
use miette::Result;

pub(crate) mod bot;
pub(crate) mod config;
pub(crate) mod db;

#[tokio::main]
async fn main() -> Result<()> {
	tracing_subscriber::fmt::init();

	let cli = Cli::parse();

	let config = config::Config::load(&cli.config).await?;

	match cli.command {
		Command::Migrate => {
			let (mut client, db_task) = config.db.connect().await?;
			let querying = tokio::spawn(db_task);
			db::migrate::migrate(&mut client).await?;
			querying.abort();
		}
		#[cfg(debug_assertions)]
		Command::ResetDb => {
			let (mut client, db_task) = config.db.connect().await?;
			let querying = tokio::spawn(db_task);
			db::migrate::drop(&client).await?;
			db::migrate::migrate(&mut client).await?;
			querying.abort();
		}
		Command::Start => {
			bot::start(config).await?;
		}
	}

	Ok(())
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
	/// Migrate database
	Migrate,

	#[cfg(debug_assertions)]
	/// Reset and then migrate database (dev only)
	ResetDb,

	/// Start bot
	Start,
}
