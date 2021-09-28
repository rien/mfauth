use chrono::serde::ts_seconds;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::ops::{Index, IndexMut};
use std::path::Path;

use crate::Opts;

#[derive(Debug, Deserialize)]
pub struct Config {
	pub accounts: HashMap<String, AccountConfig>,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct Cache {
	pub accounts: HashMap<String, Tokens>,
}

#[derive(Debug, Deserialize)]
pub struct AccountConfig {
	pub client_id: String,
	pub client_secret: String,
	pub authorize_url: String,
	pub token_url: String,
	pub scope: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Tokens {
	pub access_token: String,
	#[serde(with = "ts_seconds")]
	pub expiration: DateTime<Utc>,
	pub refresh_token: String,
}

impl Tokens {
	pub fn access_token_expired(&self) -> bool {
		self.expiration < Utc::now()
	}
}

#[derive(Debug)]
pub struct Account {
	pub conf: AccountConfig,
	pub tokens: Option<Tokens>,
}

impl Account {
	pub fn needs_refresh(&self) -> bool {
		if let Some(tokens) = &self.tokens {
			return tokens.access_token_expired();
		}
		false
	}
}

#[derive(Debug)]
pub struct Store {
	pub accounts: HashMap<String, Account>,
}

impl Store {
	pub fn read(opts: &Opts) -> std::io::Result<Self> {
		let conf_str = fs::read_to_string(&opts.config)?;
		let config: Config = toml::from_str(&conf_str).expect("config");
		let mut cache: Cache = if Path::new(&opts.store).exists() {
			let cache_str = fs::read_to_string(&opts.store)?;
			toml::from_str(&cache_str).expect("cache")
		} else {
			Cache {
				accounts: HashMap::new(),
			}
		};
		Ok(Store {
			accounts: config
				.accounts
				.into_iter()
				.map(|(name, conf)| {
					let tokens = cache.accounts.remove(&name);
					(name, Account { conf, tokens })
				})
				.collect(),
		})
	}

	pub fn write(&self, opts: &Opts) -> std::io::Result<()> {
		let cache = Cache {
			accounts: self
				.accounts
				.iter()
				.filter_map(|(name, account)| {
					account
						.tokens
						.as_ref()
						.map(|tokens| (name.to_string(), tokens.clone()))
				})
				.collect(),
		};
		let cache_str = toml::to_string(&cache).expect("cache string");
		fs::write(&opts.store, cache_str)
	}
}

impl Index<&str> for Store {
	type Output = Account;
	fn index(&self, name: &str) -> &Self::Output {
		&self.accounts[name]
	}
}

impl IndexMut<&str> for Store {
	fn index_mut(&mut self, name: &str) -> &mut Self::Output {
		self.accounts.get_mut(name).unwrap()
	}
}
