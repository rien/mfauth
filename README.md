# MFAuth

_Multi Factor Authentication for CLI mail clients_

## Installation

1. Install Rust, either with your package manager or using [rustup](https://rustup.rs/)
2. Clone this repository and enter it
  ```
  git@github.com:rien/mfauth.git
  cd mfauth
  ```
3. Build and install the program
  ```
  cargo install --path .
  ```
4. Test if it works by running `mfauth help`.

If you see a `command not found` error, you can try putting `$HOME/.cargo/bin`  in your `$PATH`.

## Usage

1. Copy the `config.example.toml` to `$HOME/.config/mfauth/config.toml` and edit the configuration if needed
2. Run `mfauth authorize <account>` to get a valid session (an access and refresh token)
3. Run `mfauth access <account>` to get a valid access token, this will automatically refresh tokens if needed
4. Configure your mail client to use `XOAUTH2` authentication and use mfauth to fetch the access tokens

### Example with msmtp

You will need to have the [`cyrus-sasl-xoauth2`](https://github.com/moriyoshi/cyrus-sasl-xoauth2) extension installed for this to work.

A working account entry in the msmtp config using mfauth:

```
account microsoft
auth xoauth2
from username@outlook.com
host outlook.office365.com
passwordeval /home/user/.cargo/bin/mfauth access microsoft
port 587
tls on
tls_starttls on
tls_trust_file /etc/ssl/certs/ca-certificates.crt
user username@outlook.com
```



