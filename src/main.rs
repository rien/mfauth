extern crate clap;
extern crate hyper;
extern crate hyper_tls;
extern crate open;
extern crate serde;
extern crate tokio;
extern crate toml;
extern crate url;

mod persist;

use clap::Clap;
use hyper::body;
use hyper::{Body, Client, Method, Request};
use hyper_tls::HttpsConnector;
use url::Url;

use std::io::{stdin, stdout, Write};

use crate::persist::*;

#[derive(Clap, Debug)]
pub struct Opts {
	#[clap(short, long)]
	verbose: bool,

	#[clap(short, long, default_value = "config.toml")]
	config: String,

	#[clap(short, long, default_value = "cache.toml")]
	store: String,

	#[clap(subcommand)]
	action: Action,
}

/// Authorize with the OAuth2 provider and return an access token
#[derive(Clap, Debug, Clone)]
enum Action {
	Authorize(Authorize),
}

/// Get an access token
#[derive(Clap, Debug, Clone)]
struct Authorize {
	/// Account to authorize
	account: String,

	/// Open your browser automatically
	#[clap(short, long)]
	open: bool,
}

#[derive(Debug)]
struct Runner {
	opts: Opts,
	store: Store,
}

#[tokio::main]
async fn main() -> std::io::Result<()> {
	let opts = Opts::parse();
	Runner::init(opts)?.run().await
}

impl Runner {
	pub fn init(opts: Opts) -> std::io::Result<Self> {
		Ok(Runner {
			store: Store::read(&opts)?,
			opts,
		})
	}

	pub fn persist(self) -> std::io::Result<()> {
		self.store.write(&self.opts)
	}

	pub async fn run(self) -> std::io::Result<()> {
		match self.opts.action.clone() {
			Action::Authorize(auth) => self.authorize(&auth.account).await,
		}
	}

	pub async fn authorize(mut self, name: &str) -> std::io::Result<()> {
		let account = &self.store[name];
		let code = self.ask_for_code(&account)?;
		let tokens = self.request_tokens(&code, &account).await?;
		self.store[name].tokens = Some(tokens);
		self.persist()
	}

	fn ask_for_code(&self, account: &Account) -> std::io::Result<String> {
		let params = [
			("response_type", "code"),
			("redirect_uri", "http://localhost"),
			("client_id", &account.conf.client_id),
			("scope", &account.conf.scope),
		];
		let url = Url::parse_with_params(&account.conf.authorize_url, &params)
			.expect("authorize url");

		println!("Visit the following link, login and paste the return URL:");
		println!(" {}", url.as_str());

		print!("URL: ");
		stdout().flush()?;
		let mut response = String::new();
		stdin().read_line(&mut response)?;
		let url = Url::parse(response.trim()).expect("Response url");
		let (_k, code) = url
			.query_pairs()
			.find(|(k, _v)| k == "code")
			.expect("code parameter");
		return Ok(code.to_string());
	}

	async fn request_tokens(
		&self,
		code: &str,
		account: &Account,
	) -> std::io::Result<Tokens> {
		let form = [
			("grant_type", "authorization_code"),
			("client_id", &account.conf.client_id),
			("redirect_uri", "http://localhost"),
			("client_secret", &account.conf.client_secret),
			("code", &code),
		];
		let body = Url::parse_with_params("http://empty/", &form)
			.expect("form url")
			.query()
			.expect("form data")
			.to_string();

		let req = Request::builder()
			.method(Method::POST)
			.uri(&account.conf.token_url)
			.body(Body::from(body))
			.expect("request builder");
		let https = HttpsConnector::new();
		let client = Client::builder().build::<_, hyper::Body>(https);

		let response = client.request(req).await.expect("request");
		let bytes = body::to_bytes(response.into_body()).await.expect("bytes");
		let response_body =
			String::from_utf8(bytes.to_vec()).expect("body utf8");
		let tokens = serde_json::from_str(&response_body).expect("tokens");
		Ok(tokens)
	}
}
