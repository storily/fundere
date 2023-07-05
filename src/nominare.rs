use std::{collections::HashMap, fmt};

use itertools::Itertools;
use reqwest::{Client, Result};
use serde::Deserialize;

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

	pub async fn random(&self, count: u8) -> Result<Vec<Name>> {
		self.client
			.get(format!("{}/random", self.url))
			.query(&param("n", count.to_string().as_str()))
			.send()
			.await?
			.error_for_status()?
			.json()
			.await
	}

	pub async fn search(&self, query: &str) -> Result<Vec<Name>> {
		self.client
			.get(format!("{}/search", self.url))
			.query(&param("q", query))
			.send()
			.await?
			.error_for_status()?
			.json()
			.await
	}

	pub async fn details(&self, name: &str) -> Result<NameDetails> {
		self.client
			.get(format!("{}/details", self.url))
			.query(&param("name", name))
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

fn param(name: &'static str, value: &str) -> HashMap<String, String> {
	let mut map = HashMap::new();
	map.insert(name.into(), value.into());
	map
}

#[derive(Deserialize, Debug, Clone)]
pub struct Name {
	#[serde(rename = "first")]
	pub given: Option<String>,

	#[serde(rename = "last")]
	pub surname: Option<Surname>,
}

impl fmt::Display for Name {
	fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
		match self {
			Self {
				given: None,
				surname: Some(name),
			} => write!(f, "{name}"),
			Self {
				given: Some(name),
				surname: None,
			} => write!(f, "{name}"),
			Self {
				given: Some(given),
				surname: Some(surname),
			} => write!(f, "{given} {surname}"),
			_ => Ok(()),
		}
	}
}

#[derive(Deserialize, Debug, Clone)]
#[serde(untagged)]
pub enum Surname {
	Single(String),
	Barrel(Vec<String>),
}

impl fmt::Display for Surname {
	fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
		match self {
			Self::Single(name) => write!(f, "{name}"),
			Self::Barrel(names) => write!(f, "{}", names.iter().join("-")),
		}
	}
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
