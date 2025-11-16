use chrono::NaiveDate;
use miette::{miette, Context, IntoDiagnostic, Result};
use reqwest::{header, Client, StatusCode};
use secret_vault_value::SecretValue;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use tracing::{debug, warn};

const API_BASE_URL: &str = "https://trackbear.app/api/v1";

fn user_agent() -> String {
	format!(
		"{}/{} ({}) by {}",
		env!("CARGO_PKG_NAME"),
		env!("CARGO_PKG_VERSION"),
		env!("CARGO_PKG_REPOSITORY"),
		env!("CARGO_PKG_AUTHORS")
	)
}

#[derive(Debug, Clone)]
pub struct TrackbearClient {
	client: Client,
	api_key: SecretValue,
}

#[derive(Debug, Deserialize)]
pub struct PingResponse {
	pub pong: String,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Balance {
	pub word: i64,
	pub time: i64,
	pub page: i64,
	pub chapter: i64,
	pub scene: i64,
	pub line: i64,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Project {
	pub id: i64,
	pub uuid: String,
	pub created_at: String,
	pub updated_at: String,
	pub state: String,
	pub owner_id: i64,
	pub title: String,
	pub description: String,
	pub phase: String,
	pub starting_balance: Balance,
	pub cover: String,
	pub starred: bool,
	pub display_on_profile: bool,
	pub totals: Balance,
	pub last_updated: String,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GoalThreshold {
	pub measure: String,
	pub count: i64,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GoalParameters {
	pub threshold: GoalThreshold,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Goal {
	pub id: i64,
	pub uuid: String,
	pub created_at: String,
	pub updated_at: String,
	pub state: String,
	pub owner_id: i64,
	pub title: String,
	pub description: String,
	#[serde(rename = "type")]
	pub goal_type: String,
	pub parameters: GoalParameters,
	pub start_date: String,
	pub end_date: String,
	pub work_ids: Vec<i64>,
	pub tag_ids: Vec<i64>,
	pub starred: bool,
	pub display_on_profile: bool,
	pub achieved: bool,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Tag {
	pub id: i64,
	pub uuid: String,
	pub created_at: String,
	pub updated_at: String,
	pub state: String,
	pub owner_id: i64,
	pub name: String,
	pub color: String,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TallyWork {
	pub id: i64,
	pub uuid: String,
	pub created_at: String,
	pub updated_at: String,
	pub state: String,
	pub owner_id: i64,
	pub title: String,
	pub description: String,
	pub phase: String,
	pub starting_balance: Balance,
	pub cover: String,
	pub starred: bool,
	pub display_on_profile: bool,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Tally {
	pub id: i64,
	pub uuid: String,
	pub created_at: String,
	pub updated_at: String,
	pub state: String,
	pub owner_id: i64,
	pub date: String,
	pub measure: String,
	pub count: i64,
	pub note: String,
	pub work_id: i64,
	pub work: TallyWork,
	pub tags: Vec<Tag>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateTallyRequest {
	pub date: String,
	pub measure: String,
	pub count: i64,
	pub note: String,
	pub work_id: i64,
	pub set_total: bool,
	pub tags: Vec<String>,
}

impl TrackbearClient {
	/// Create a new TrackBear client with an API key
	pub fn new(api_key: SecretValue) -> Result<Self> {
		let mut headers = header::HeaderMap::new();

		// Add User-Agent header
		let user_agent_value = user_agent();
		headers.insert(
			header::USER_AGENT,
			header::HeaderValue::from_str(&user_agent_value)
				.into_diagnostic()
				.wrap_err("invalid user agent format")?,
		);

		// Add Authorization header with Bearer token
		let auth_value = format!("Bearer {}", api_key.as_sensitive_str());
		headers.insert(
			header::AUTHORIZATION,
			header::HeaderValue::from_str(&auth_value)
				.into_diagnostic()
				.wrap_err("invalid API key format")?,
		);

		let client = Client::builder()
			.default_headers(headers)
			.build()
			.into_diagnostic()
			.wrap_err("failed to build HTTP client")?;

		Ok(Self { client, api_key })
	}

	/// Test API connectivity without authentication
	pub async fn ping_service() -> Result<bool> {
		let client = Client::builder()
			.user_agent(user_agent())
			.build()
			.into_diagnostic()
			.wrap_err("failed to build HTTP client")?;

		let url = format!("{}/ping", API_BASE_URL);
		debug!("pinging TrackBear service at {}", url);

		match client.get(&url).send().await {
			Ok(response) => {
				let status = response.status();
				debug!("service ping response: {}", status);
				Ok(status.is_success())
			}
			Err(err) => {
				warn!("service ping failed: {}", err);
				Ok(false)
			}
		}
	}

	/// Test API key validity by pinging with authentication
	pub async fn ping_with_token(&self) -> Result<()> {
		let url = format!("{}/ping/api-token", API_BASE_URL);
		debug!("pinging TrackBear API with token");

		let response = self
			.client
			.get(&url)
			.send()
			.await
			.into_diagnostic()
			.wrap_err("failed to connect to TrackBear API")?;

		let status = response.status();
		debug!("API token ping response: {}", status);

		match status {
			StatusCode::OK => {
				let _ping: PingResponse = response
					.json()
					.await
					.into_diagnostic()
					.wrap_err("failed to parse ping response")?;
				debug!("API key is valid");
				Ok(())
			}
			StatusCode::UNAUTHORIZED | StatusCode::FORBIDDEN => Err(miette!(
				"Invalid API key - please check your TrackBear API key"
			)),
			_ => {
				let error_text = response
					.text()
					.await
					.unwrap_or_else(|_| "unknown error".to_string());
				Err(miette!(
					"TrackBear API returned error (status {}): {}",
					status,
					error_text
				))
			}
		}
	}

	/// Validate the API key - checks token first, then service if token fails
	pub async fn validate(&self) -> Result<()> {
		match self.ping_with_token().await {
			Ok(()) => Ok(()),
			Err(err) => {
				// If the token ping fails, check if the service is up
				if Self::ping_service().await? {
					// Service is up, so the token is invalid
					Err(err)
				} else {
					// Service is down
					Err(miette!(
						"TrackBear service appears to be unavailable. Please try again later."
					))
				}
			}
		}
	}

	/// Get the base URL for the API
	pub fn base_url() -> &'static str {
		API_BASE_URL
	}

	/// Get the underlying HTTP client
	pub fn http_client(&self) -> &Client {
		&self.client
	}

	/// List all projects
	pub async fn list_projects(&self) -> Result<Vec<Project>> {
		let url = format!("{}/project", API_BASE_URL);
		debug!("fetching projects from TrackBear");

		let response = self
			.client
			.get(&url)
			.send()
			.await
			.into_diagnostic()
			.wrap_err("failed to connect to TrackBear API")?;

		let status = response.status();
		debug!("list projects response: {}", status);

		if !status.is_success() {
			let error_text = response
				.text()
				.await
				.unwrap_or_else(|_| "unknown error".to_string());
			return Err(miette!(
				"TrackBear API returned error (status {}): {}",
				status,
				error_text
			));
		}

		response
			.json::<Vec<Project>>()
			.await
			.into_diagnostic()
			.wrap_err("failed to parse projects response")
	}

	/// List all goals
	pub async fn list_goals(&self) -> Result<Vec<Goal>> {
		let url = format!("{}/goal", API_BASE_URL);
		debug!("fetching goals from TrackBear");

		let response = self
			.client
			.get(&url)
			.send()
			.await
			.into_diagnostic()
			.wrap_err("failed to connect to TrackBear API")?;

		let status = response.status();
		debug!("list goals response: {}", status);

		if !status.is_success() {
			let error_text = response
				.text()
				.await
				.unwrap_or_else(|_| "unknown error".to_string());
			return Err(miette!(
				"TrackBear API returned error (status {}): {}",
				status,
				error_text
			));
		}

		response
			.json::<Vec<Goal>>()
			.await
			.into_diagnostic()
			.wrap_err("failed to parse goals response")
	}

	/// List tallies with optional filters
	pub async fn list_tallies(
		&self,
		work_ids: Option<&[i64]>,
		tag_ids: Option<&[i64]>,
		measure: Option<&str>,
		start_date: Option<NaiveDate>,
		end_date: Option<NaiveDate>,
	) -> Result<Vec<Tally>> {
		let mut url = format!("{}/tally", API_BASE_URL);
		let mut query_params = Vec::new();

		if let Some(works) = work_ids {
			for work_id in works {
				query_params.push(format!("works[]={}", work_id));
			}
		}

		if let Some(tags) = tag_ids {
			for tag_id in tags {
				query_params.push(format!("tags[]={}", tag_id));
			}
		}

		if let Some(m) = measure {
			query_params.push(format!("measure={}", m));
		}

		if let Some(start) = start_date {
			query_params.push(format!("startDate={}", start.format("%Y-%m-%d")));
		}

		if let Some(end) = end_date {
			query_params.push(format!("endDate={}", end.format("%Y-%m-%d")));
		}

		if !query_params.is_empty() {
			url.push('?');
			url.push_str(&query_params.join("&"));
		}

		debug!("fetching tallies from TrackBear: {}", url);

		let response = self
			.client
			.get(&url)
			.send()
			.await
			.into_diagnostic()
			.wrap_err("failed to connect to TrackBear API")?;

		let status = response.status();
		debug!("list tallies response: {}", status);

		if !status.is_success() {
			let error_text = response
				.text()
				.await
				.unwrap_or_else(|_| "unknown error".to_string());
			return Err(miette!(
				"TrackBear API returned error (status {}): {}",
				status,
				error_text
			));
		}

		response
			.json::<Vec<Tally>>()
			.await
			.into_diagnostic()
			.wrap_err("failed to parse tallies response")
	}

	/// Create a new tally
	pub async fn create_tally(&self, request: CreateTallyRequest) -> Result<Tally> {
		let url = format!("{}/tally", API_BASE_URL);
		debug!("creating tally on TrackBear: {:?}", request);

		let response = self
			.client
			.post(&url)
			.json(&request)
			.send()
			.await
			.into_diagnostic()
			.wrap_err("failed to connect to TrackBear API")?;

		let status = response.status();
		debug!("create tally response: {}", status);

		if !status.is_success() {
			let error_text = response
				.text()
				.await
				.unwrap_or_else(|_| "unknown error".to_string());
			return Err(miette!(
				"TrackBear API returned error (status {}): {}",
				status,
				error_text
			));
		}

		response
			.json::<Tally>()
			.await
			.into_diagnostic()
			.wrap_err("failed to parse tally response")
	}
}

#[cfg(test)]
mod tests {
	use super::*;

	#[tokio::test]
	async fn test_ping_service() {
		// This should work without authentication
		let result = TrackbearClient::ping_service().await;
		assert!(result.is_ok());
	}

	#[tokio::test]
	async fn test_invalid_token() {
		let client = TrackbearClient::new(SecretValue::from("invalid-token")).unwrap();
		let result = client.ping_with_token().await;
		assert!(result.is_err());
	}
}
