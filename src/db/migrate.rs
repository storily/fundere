use miette::{miette, Context, IntoDiagnostic, Result};
use tokio_postgres::Client;
use tracing::info;

macro_rules! migration {
	($name:expr) => {
		(
			$name,
			include_str!(concat!("../../migrations/", $name, ".sql")),
		)
	};
}

const MIGRATIONS: &[(&str, &str)] = &[
	migration!("000_base"),
	migration!("001_sprints"),
	migration!("002_errors"),
];

#[cfg(debug_assertions)]
#[tracing::instrument(skip(db))]
pub async fn drop(db: &Client) -> Result<()> {
	db.batch_execute("DROP SCHEMA public CASCADE; CREATE SCHEMA public")
		.await
		.into_diagnostic()
}

#[tracing::instrument(skip(db))]
pub async fn migrate(db: &mut Client) -> Result<()> {
	let first_todo = if let Some(last) = last_migration(db).await {
		MIGRATIONS
			.iter()
			.enumerate()
			.find_map(|(n, (name, _))| if &&last == name { Some(n + 1) } else { None })
			.ok_or_else(|| {
				miette!("last migration applied is not in available set, database in invalid state")
			})?
	} else {
		0
	};

	for (name, migration) in &MIGRATIONS[first_todo..] {
		info!(?name, "applying migration");
		apply_migration(db, *name, *migration)
			.await
			.wrap_err_with(|| format!("migrating {name}"))?;
	}

	Ok(())
}

#[tracing::instrument(skip(db))]
async fn last_migration(db: &Client) -> Option<String> {
	db.query_one("SELECT name FROM migrations ORDER BY n DESC LIMIT 1", &[])
		.await
		.and_then(|row| row.try_get("name"))
		.ok()
}

#[tracing::instrument(skip(db))]
async fn apply_migration(db: &mut Client, name: &str, query: &str) -> Result<()> {
	let txn = db.transaction().await.into_diagnostic()?;
	txn.batch_execute(query).await.into_diagnostic()?;
	txn.query("INSERT INTO migrations (name) VALUES ($1)", &[&name])
		.await
		.into_diagnostic()?;
	txn.commit().await.into_diagnostic()
}
