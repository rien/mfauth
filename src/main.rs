extern crate anyhow;
#[macro_use]
extern crate clap;
extern crate dirs;
extern crate hyper;
extern crate hyper_tls;
extern crate serde;
extern crate tokio;
extern crate toml;
extern crate url;

mod persist;

use anyhow::{anyhow, Context, Result};
use chrono::{Duration, Utc};
use clap::Parser;
use hyper::body;
use hyper::{Body, Client, Method, Request};
use hyper_tls::HttpsConnector;
use serde::Deserialize;
use url::Url;

use std::borrow::Borrow;
use std::io::{stdin, stdout, Write};
use std::path::PathBuf;

use crate::persist::*;

#[derive(Parser, Debug)]
pub struct Opts {
	#[clap(short, long)]
	verbose: bool,

	#[clap(short, long)]
	config: Option<PathBuf>,

	#[clap(long)]
	cache: Option<PathBuf>,

	#[clap(subcommand)]
	action: Action,
}

/// Authorize with the OAuth2 provider and return an access token
#[derive(Subcommand, Debug, Clone)]
enum Action {
	/// Authorize an account (fetch an access and refresh token)
	Authorize {
		/// Account to authorize
		account: String,
	},
	/// Get an access token
	Access {
		/// Account to get an access token for
		account: String,
	},
}

#[derive(Debug)]
struct Runner {
	opts: Opts,
	store: Store,
}

#[tokio::main]
async fn main() -> Result<()> {
	let opts = Opts::parse();
	Runner::init(opts)?.run().await
}

impl Runner {
	pub fn init(opts: Opts) -> Result<Self> {
		Ok(Runner {
			store: Store::read(&opts)?,
			opts,
		})
	}

	pub fn persist(&self) -> Result<()> {
		self.store.write()
	}

	pub async fn run(self) -> Result<()> {
		match self.opts.action.clone() {
			Action::Authorize { account } => self.authorize(&account).await,
			Action::Access { account } => self.get_access_token(&account).await,
		}
	}

	/// Prints a valid access token. If the access token in the cache is still
	/// valid, we re-use that token. If not, we try to use our refresh token to
	/// request a new pair of tokens.
	pub async fn get_access_token(mut self, name: &str) -> Result<()> {
		let account = self.store.get(name)?;
		if account.needs_refresh() {
			let tokens = self.refresh_access_token(account).await?;
			self.store.get_mut(name)?.tokens = Some(tokens);
			self.persist()?;
		}
		let account = &self.store.get(name)?;
		let tokens = account
			.tokens
			.as_ref()
			.ok_or(anyhow!("This account does not have tokens yet. You may need to run the authorize command first."))?;
		println!("{}", tokens.access_token);
		Ok(())
	}

	/// Authorize with the given provider, stores the access and refresh tokens
	/// into the cache.
	pub async fn authorize(mut self, name: &str) -> Result<()> {
		let account = &self.store.get(name)?;
		let code = self.ask_for_code(&account)?;
		let tokens = self.use_authorize_code(&code, &account).await?;
		self.store.get_mut(name)?.tokens = Some(tokens);
		self.persist()?;
		println!("Authorization OK!");
		Ok(())
	}

	/// Send the user to the authorize page and ask for the redirected URL,
	/// this URL contains a authorization code which we can use to request
	/// access and refresh tokens.
	fn ask_for_code(&self, account: &Account) -> Result<String> {
		let params = [
			("response_type", "code"),
			("redirect_uri", "http://localhost"),
			("client_id", &account.conf.client_id),
			("scope", &account.conf.scope),
		];
		let url = Url::parse_with_params(&account.conf.authorize_url, &params)
			.with_context(|| {
				format!(
					"Could not parse the authorize URL: {}",
					&account.conf.authorize_url
				)
			})?;

		println!("Visit the following link, login and paste the return URL:");
		println!(" {}", url.as_str());

		print!("URL: ");
		stdout().flush()?;
		let mut response = String::new();
		stdin().read_line(&mut response)?;
		let url = Url::parse(response.trim())
			.context("Could not parse the return URL you pasted.")?;
		let (_k, code) = url
			.query_pairs()
			.find(|(k, _v)| k == "code")
			.ok_or(anyhow!("No code parameter in response link"))?;
		return Ok(code.to_string());
	}

	/// Use the refresh token to request new pair of tokens
	async fn refresh_access_token(&self, account: &Account) -> Result<Tokens> {
		let refresh_token = &account
			.tokens
			.as_ref()
			.ok_or(anyhow!("No tokens to refresh"))?
			.refresh_token;
		let form = [
			("grant_type", "refresh_token"),
			("client_id", &account.conf.client_id),
			("client_secret", &account.conf.client_secret),
			("refresh_token", refresh_token),
		];
		Self::request_tokens(form, &account.conf.token_url).await
	}

	/// Use an authorization code to request an access and refresh token
	async fn use_authorize_code(
		&self,
		code: &str,
		account: &Account,
	) -> Result<Tokens> {
		let form = [
			("grant_type", "authorization_code"),
			("client_id", &account.conf.client_id),
			("redirect_uri", "http://localhost"),
			("client_secret", &account.conf.client_secret),
			("code", &code),
		];
		Self::request_tokens(form, &account.conf.token_url).await
	}

	/// Send a POST request to the token URL requesting new tokens
	async fn request_tokens<I, K, V>(form: I, token_url: &str) -> Result<Tokens>
	where
		I: IntoIterator,
		I::Item: Borrow<(K, V)>,
		K: AsRef<str>,
		V: AsRef<str>,
	{
		let body = Url::parse_with_params("http://empty/", form)?
			.query()
			.unwrap()
			.to_string();

		let req = Request::builder()
			.method(Method::POST)
			.uri(token_url)
			.body(Body::from(body))?;
		let https = HttpsConnector::new();
		let client = Client::builder().build::<_, hyper::Body>(https);

		let response = client.request(req).await?;
		let status = response.status();

		let bytes = body::to_bytes(response.into_body()).await?;
		let response_body = String::from_utf8(bytes.to_vec())?;

		if status.is_success() {
			#[derive(Deserialize)]
			struct TokenResponse {
				access_token: String,
				refresh_token: String,
				expires_in: i64,
			}

			let tokens: TokenResponse = serde_json::from_str(&response_body)
				.with_context(|| {
					format!(
						"Could not parse succesful token response: {}",
						response_body
					)
				})?;
			let expiration = Utc::now() + Duration::seconds(tokens.expires_in);

			Ok(Tokens {
				access_token: tokens.access_token,
				refresh_token: tokens.refresh_token,
				expiration,
			})
		} else {
			#[derive(Deserialize)]
			struct ErrorResponse {
				error: String,
				error_description: String,
			}

			let error: ErrorResponse = serde_json::from_str(&response_body)
				.with_context(|| {
					format!(
						"Could not parse token response with status {}: {}",
						status, response_body
					)
				})?;

			Err(anyhow!(
				"Server returned an {} error.\n{}",
				error.error,
				error.error_description
			))
		}
	}
}
