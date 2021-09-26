extern crate clap;
extern crate hyper;
extern crate hyper_tls;
extern crate open;
extern crate serde;
extern crate tokio;
extern crate toml;
extern crate url;

use clap::Clap;
use hyper::body;
use hyper::{Body, Client, Method, Request, Response, Uri};
use hyper_tls::HttpsConnector;
use serde::{Deserialize, Serialize};
use url::Url;

use std::collections::HashMap;
use std::fs;
use std::io::{stdin, stdout, Write};

#[derive(Clap, Debug)]
struct Opts {
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
#[derive(Clap, Debug)]
enum Action {
	Authorize(Authorize),
}

/// Get an access token
#[derive(Clap, Debug)]
struct Authorize {
	/// Account to authorize
	account: String,

	/// Open your browser automatically
	#[clap(short, long)]
	open: bool,
}

#[derive(Debug, Deserialize)]
struct Config {
	accounts: HashMap<String, Account>,
}

#[derive(Debug, Deserialize)]
struct Account {
	client_id: String,
	client_secret: String,
	authorize_url: String,
	token_url: String,
	scope: String,
}

#[derive(Debug, Deserialize, Serialize)]
struct Cache {
	accounts: HashMap<String, Tokens>,
}

#[derive(Debug, Serialize, Deserialize)]
struct Tokens {
	access_token: String,
	refresh_token: String,
}

#[tokio::main]
async fn main() -> std::io::Result<()> {
	let opts = Opts::parse();
	let conf_str = fs::read_to_string(&opts.config)?;
	let conf: Config = toml::from_str(&conf_str).expect("config");
	match opts.action {
		Action::Authorize(auth) => authorize(&auth.account, &conf).await,
	}
}

async fn authorize(account_name: &str, conf: &Config) -> std::io::Result<()> {
	let account = &conf.accounts[account_name];
	let code = dbg!(ask_for_code(account)?);
	let tokens = dbg!(request_tokens(&code, account).await?);
	Ok(())
}

fn ask_for_code(account: &Account) -> std::io::Result<String> {
	let params = [
		("response_type", "code"),
		("redirect_uri", "http://localhost"),
		("client_id", &account.client_id),
		("scope", &account.scope),
	];
	let url = Url::parse_with_params(&account.authorize_url, &params)
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
	code: &str,
	account: &Account,
) -> std::io::Result<Tokens> {
	let form = [
		("grant_type", "authorization_code"),
		("client_id", &account.client_id),
		("redirect_uri", "http://localhost"),
		("client_secret", &account.client_secret),
		("code", &code),
	];
	let body = Url::parse_with_params("http://empty/", &form)
		.expect("form url")
		.query()
		.expect("form data")
		.to_string();

	let req = Request::builder()
		.method(Method::POST)
		.uri(&account.token_url)
		.body(Body::from(body))
		.expect("request builder");
	let https = HttpsConnector::new();
	let client = Client::builder().build::<_, hyper::Body>(https);

	let response = dbg!(client.request(req).await.expect("request"));
	let bytes = body::to_bytes(response.into_body()).await.expect("bytes");
	let response_body = String::from_utf8(bytes.to_vec()).expect("body utf8");
	let tokens = serde_json::from_str(&response_body).expect("tokens");
	Ok(tokens)
}
