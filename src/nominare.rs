use std::collections::HashMap;

use reqwest::{Client, Result};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone)]
pub struct Nominare {
	url: String,
	client: Client,
}

impl Nominare {
	pub fn new(url: &str) -> Self {
		Self {
			url: url.into(),
			client: Client::new(),
		}
	}

	pub async fn random(&self, count: u8) -> Result<Vec<String>> {
		self.client
			.get(format!("{}/random", self.url))
			.query(&Params::RandomCount(count))
			.send()
			.await?
			.error_for_status()?
			.json()
			.await
	}

	pub async fn search(&self, query: &str) -> Result<Vec<String>> {
		self.client
			.get(format!("{}/search", self.url))
			.query(&Params::SearchQuery(query))
			.send()
			.await?
			.error_for_status()?
			.json()
			.await
	}

	pub async fn details(&self, name: &str) -> Result<NameDetails> {
		self.client
			.get(format!("{}/details", self.url))
			.query(&Params::Details(name))
			.send()
			.await?
			.error_for_status()?
			.json()
			.await
	}

	pub async fn stats(&self) -> Result<Stats> {
		self.client
			.get(format!("{}/stats", self.url))
			.send()
			.await?
			.error_for_status()?
			.json()
			.await
	}
}

#[derive(Serialize)]
enum Params<'a> {
	#[serde(rename = "n")]
	RandomCount(u8),

	#[serde(rename = "q")]
	SearchQuery(&'a str),

	#[serde(rename = "name")]
	Details(&'a str),
}

#[derive(Deserialize, Debug, Clone)]
pub struct Stats {
	pub total: u64,
	pub firsts: u64,
	pub lasts: u64,
	pub genders: Genders,
	pub kinds: HashMap<String, u64>,
}

#[derive(Deserialize, Debug, Clone)]
pub struct Genders {
	pub male: u64,
	pub female: u64,
	pub enby: u64,
}

#[derive(Deserialize, Debug, Clone)]
pub struct NameDetails {
	#[serde(rename = "first")]
	pub given: NameDetail,

	#[serde(rename = "last")]
	pub surname: NameDetail,
}

#[derive(Deserialize, Debug, Clone)]
pub struct NameDetail {
	pub name: String,
	pub kinds: Vec<String>,
	pub sources: Vec<String>,
	pub score: f64,
	pub gender: Gender,
}

#[derive(Deserialize, Debug, Clone)]
pub enum Gender {
	#[serde(rename = "male")]
	Male,
	#[serde(rename = "female")]
	Female,
	#[serde(rename = "enby")]
	Enby,
}
