use miette::{miette, Context, IntoDiagnostic, Result};
use reqwest::{header, Client, StatusCode};
use secret_vault_value::SecretValue;
use serde::Deserialize;
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

#[derive(Clone)]
pub struct TrackbearClient {
	client: Client,
	api_key: SecretValue,
}

#[derive(Debug, Deserialize)]
pub struct PingResponse {
	pub pong: String,
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
